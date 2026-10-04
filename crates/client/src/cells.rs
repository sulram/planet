//! The cells of a world on the client: volumes seated on the planet and on
//! the moon, what changes them, and the meshes that draw them (DECISIONS
//! 106).
//!
//! A volume knows no sphere (`voxel`). Here it is seated, and the address
//! cuts it (DECISIONS 77): a sector's columns are cut into plots of
//! [`PLOT_BITS`], a volume stands over one plot, and its cell `(x, y, z)` is
//! the address `(u, v, h)` of that sector. So a cube is exactly a block of
//! the world, neighbours meet with nothing between them, and a body walks on
//! them in address space the way it walks on the ground (CLAUDE.md, simulate
//! flat, render spherical). Each corner of each side is bent onto its body
//! on its own, which is why the sides are never merged.
//!
//! A seat of the moon is counted on the moon's own grid, from the moon's
//! centre: what is built there is meshed around that centre, and whoever
//! draws it, aims at it or lights with it adds where the moon is now.
//!
//! The ground under a volume stays as nature made it (DECISIONS 78).
//!
//! Past holding distance a volume is held as it is seen from afar
//! (`voxel::afar`): a cell for every few each way, drawn as cubes as wide,
//! with no edge, no lamp and nothing to stand on or aim at. A volume left
//! behind is made so here from what was held of it, and one never held
//! comes so from the world. Come near again, it is drawn afar until it is
//! drawn whole.
//!
//! The cells are a system of the core (DECISIONS 108): anyone reads what
//! they hold, and a gesture alone changes it. How a hand arrives at gestures,
//! a tool, a stroke, a platform, is a plugin's, and it speaks to the cells
//! through its host (`crate::plugin`): aim, preview, apply, take back.
//!
//! In a world, the world keeps the cells (DECISIONS 107, 110). A change is
//! made here at once and asked of the world, which answers that it landed or
//! why not; what others near change arrives as events, and a volume the
//! body comes near is asked for whole. Until the world answers, a change is
//! pending: what others changed meanwhile goes under it, and a refusal
//! takes it away. With no world, a headless picture or a bench, the cells
//! are held here alone.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use glam::{DVec3, Vec3};
use protocol::cells as wire;
use protocol::{Body, Message};
use scene::{
    GuideMesh, GuideVertex, Lamp, VolumeChange, VolumeDraw, VolumeMesh, VolumeMeshId, VolumeVertex,
};
use seat::{AFAR_DROP_M, DROP_M, PLOT_BITS, Stand, Unseated, plot_of};
use topology::{BLOCK_M, QuadSphere, Sector, SurfacePoint};
use voxel::{
    AFAR, CHUNK, CHUNK_AFAR, CHUNK_AFAR_CELLS, CHUNK_BITS, Cell, Edge, Face, Finish, Gesture, Hit,
    Paint, Quad, Span, Volumes, crossing, unpack,
};
use worldgen::{Generator, Source};

pub use seat::Seat;
pub(crate) use seat::ground;

use crate::collision::{Footing, STEP_M};
use crate::grass::TUFT_M;

/// Metres between the points of a line of sight where the ground is asked
/// whether it hides what the pointer is over.
const GROUND_STEP_M: f64 = 1.0;
/// How far a pointer reaches into the world to build, metres.
const REACH_M: f64 = 80.0;
/// Metres between the points of a line of sight bent into a volume's cells:
/// a block, straight enough to walk exactly between them.
const SIGHT_STEP_M: f64 = BLOCK_M;
/// Times the stretch of a line of sight that crosses the ground is halved
/// to find where: to a sixteenth of a cell.
const GROUND_HALVINGS: usize = 5;
/// Half the width of a standing body in address space, in cells: how near a
/// wall it comes.
const BODY_HALF: f64 = 0.6;
/// How far the ghost of a gesture stands off the cubes it covers, metres.
const GHOST_LIFT_M: f32 = 0.012;
/// What marks the mesh of a chunk of a volume seen from afar.
const AFAR_MESH: u64 = 1 << 61;
/// The ghost's id, which no chunk of any volume has.
const GHOST: VolumeMeshId = VolumeMeshId(u64::MAX);
/// The first guide's id: the next ones count down from it.
const GUIDES: u64 = u64::MAX - 1;
/// What marks the mesh of a chunk's glass, beside the mesh of its cubes.
const GLASS: u64 = 1 << 62;
/// Cells along a side of the quads a guide is made of, at most: few enough
/// that its sides bend with the planet as the cells they show do.
const GUIDE_STEP: i32 = 8;
/// How far a guide stands off the box it shows, in cells: clear of the cubes
/// that fill the box to its sides.
const GUIDE_LIFT: f64 = 0.04;
/// What a guide in no paint is drawn in: room to build in, barely there
/// between its lines. A pale line with a dark rim each side of it, so it
/// shows over grass and over the pale ground of the moon, by day and by
/// night.
const ROOM_COLOR: [u8; 3] = [255, 255, 255];
const ROOM_FILL: u8 = 3;
const ROOM_INK: u8 = 56;
const ROOM_RIM: [u8; 4] = [0, 0, 0, 64];
/// How much shows of what a hand would make, of 255, between its lines and
/// on them: plain to see, and the world still seen through it.
const GHOST_FILL: u8 = 105;
const GHOST_INK: u8 = 170;
/// How brightly a lamp of one side shines on what faces it from a metre
/// away, as a share of its colour: a lamp of more sides shines by the root
/// of how many, up to [`LAMP_SIDES`].
const LAMP_GAIN: f32 = 0.7;
const LAMP_SIDES: f32 = 64.0;
/// How far a lamp of one side reaches, metres, how much further for the root
/// of each side more, and the furthest any does.
const LAMP_REACH_M: f32 = 8.0;
const LAMP_GROWS_M: f32 = 3.0;
const LAMP_FAR_M: f32 = 32.0;
/// Changes kept to take back.
const HISTORY: usize = 100;
/// Seconds between two looks at what the world holds near the body.
const LOOK_S: f64 = 1.0;
/// Cells in a chunk.
const CHUNK_CELLS: usize = (CHUNK * CHUNK * CHUNK) as usize;
/// Chunks meshed in one update, the nearest the eye first. A change to more
/// is drawn over as many updates as it takes, each chunk keeping the mesh it
/// had until its new one is made, so no frame pays for a platform whole.
const MESHES_PER_UPDATE: usize = 8;

/// The paints a cell can take, as sRGB: whites and greys, earth, green, water
/// and a few loud ones.
pub const PALETTE: [[u8; 3]; 16] = [
    [242, 240, 235],
    [184, 184, 179],
    [92, 92, 90],
    [30, 30, 31],
    [156, 59, 46],
    [200, 105, 63],
    [217, 192, 140],
    [122, 82, 48],
    [95, 122, 58],
    [63, 143, 74],
    [47, 140, 140],
    [90, 167, 224],
    [42, 63, 122],
    [124, 82, 184],
    [232, 122, 168],
    [240, 201, 58],
];

/// What a gesture that deletes shows over what it takes away.
const DELETE_COLOR: [u8; 3] = [230, 70, 60];

/// Why the cells refuse what was asked of them. A plugin says it at the
/// seam under its own name, and a front end in its own words.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Refusal {
    /// Changing the cells is a builder's and an admin's, and the world named
    /// this session a lower level, or no world has spoken (DECISIONS 104).
    Level,
    /// The world is shaped by a field, and its server does not hold the
    /// field to say where a volume starts and ends (OPEN).
    Field,
    /// The ground here is under the sea.
    Sea,
    /// The plot is on the edge of a sector: a volume stays inside one.
    Seam,
}

/// A box of a seat's cells shown as a faint grid, a line between each cell
/// and the next: where cells are, before any is laid. In a paint, what a
/// hand would lay there. In none, room to build in.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Guide {
    pub seat: Seat,
    pub span: Span,
    pub paint: Option<u8>,
}

/// The volumes of one seat: a cell is the address it has there. Every place
/// it speaks of is counted from the centre of the body the seat is on.
struct Seated {
    seat: Seat,
    /// The body the seat is on: the grid its cells are cut by, and the
    /// radius that turns them into metres.
    sphere: QuadSphere,
    /// Cells of the seat along each side of a cell of these volumes: 1, the
    /// cells themselves, or [`AFAR`], the volumes as they are seen from afar.
    scale: i32,
    volumes: Volumes,
    /// A ball around each volume, by its plot: what a line of sight asks
    /// before it is bent into cells. Then how far its plot reaches from its
    /// middle, along the ground.
    balls: BTreeMap<[i32; 2], (DVec3, f64, f64)>,
}

impl Seated {
    /// Where a lattice point of these volumes' cells is.
    fn corner(&self, p: [i32; 3]) -> DVec3 {
        self.at(p.map(|n| f64::from(n * self.scale)))
    }

    /// Whether these are the volumes as they are seen from afar.
    fn afar(&self) -> bool {
        self.scale != 1
    }

    /// Where a point of the seat's cells is.
    fn at(&self, p: [f64; 3]) -> DVec3 {
        let point = SurfacePoint::new(self.seat.sector(), p[0], p[1]);
        DVec3::from(self.sphere.position(point, p[2] * BLOCK_M))
    }

    /// A place in this seat's cells, when it is over its sector.
    fn cells_of(&self, p: DVec3) -> Option<[f64; 3]> {
        let point = self.sphere.blocks().surface_point(p.to_array());
        (point.sector == self.seat.sector()).then(|| {
            [
                point.u,
                point.v,
                (p.length() - self.sphere.radius_m()) / BLOCK_M,
            ]
        })
    }

    /// Opens the volume of a plot, holding the blocks from `low` to `height`
    /// over it.
    fn open(&mut self, plot: [i32; 2], low: i32, height: u32) {
        if !self.volumes.open(plot, low, height) {
            return;
        }
        let Some(bounds) = self.volumes.bounds(plot) else {
            return;
        };
        let size = bounds.size().map(|n| n as i32);
        let half = [0, 1, 2].map(|i| bounds.min[i] + size[i] / 2);
        let middle = self.corner(half);
        let radius_m = self.corner(bounds.min).distance(middle) * 1.5;
        let level = [bounds.min[0], bounds.min[1], half[2]];
        let across_m = self.corner(level).distance(middle) * 1.5;
        self.balls.insert(plot, (middle, radius_m, across_m));
    }

    /// A line of sight from `from` along `toward` (unit), as a path in this
    /// seat's cells: the stretch of it that could reach a volume, and how
    /// far along the line it ends.
    fn sight(&self, from: DVec3, toward: DVec3) -> (f64, Vec<[f64; 3]>) {
        let stretch = self
            .balls
            .values()
            .filter_map(|&(middle, radius_m, _)| {
                let along = (middle - from).dot(toward);
                let miss2 = (middle - from).length_squared() - along * along;
                let inside2 = radius_m * radius_m - miss2;
                (inside2 >= 0.0).then(|| (along - inside2.sqrt(), along + inside2.sqrt()))
            })
            .reduce(|a, b| (a.0.min(b.0), a.1.max(b.1)));
        let Some((near, far)) = stretch else {
            return (0.0, Vec::new());
        };
        let (near, far) = (near.max(0.0), far.min(REACH_M));
        let steps = ((far - near) / SIGHT_STEP_M).ceil().max(0.0) as usize;
        let path = (0..=steps).filter_map(|i| {
            let t = (near + i as f64 * SIGHT_STEP_M).min(far);
            self.cells_of(from + toward * t)
        });
        (far, path.collect())
    }

    /// Where the middle of the side a line came in through is.
    fn side(&self, hit: Hit) -> DVec3 {
        let axis = hit.face.axis;
        let mut middle = hit.cell.map(|n| f64::from(n) + 0.5);
        middle[axis] = f64::from(hit.cell[axis] + i32::from(hit.face.positive));
        self.at(middle)
    }

    /// The cell that holds the ground under a place, when the place is over
    /// this seat's sector and a volume holds that cell.
    fn ground_cell(&self, generator: &Generator, p: DVec3) -> Option<[i32; 3]> {
        let [u, v, _] = self.cells_of(p)?;
        let point = SurfacePoint::new(self.seat.sector(), u, v);
        let cell = [u, v, ground(generator, self.seat, point)].map(|at| at.floor() as i32);
        self.volumes.holds(cell).then_some(cell)
    }

    /// The mesh of a chunk, named by the address of its lowest corner.
    fn mesh_id(&self, chunk: [i32; 3]) -> VolumeMeshId {
        let [u, v, h] = chunk;
        let across = |n: i32| u64::from(n as u32 >> CHUNK_BITS);
        // Heights run either side of the datum: 24 bits hold them all.
        let up = u64::from((h + (1 << 23)) as u32 & 0xff_ffff);
        let afar = if self.afar() { AFAR_MESH } else { 0 };
        VolumeMeshId(
            afar | u64::from(self.seat.key()) << 56 | across(u) << 40 | across(v) << 24 | up,
        )
    }
}

/// What a line of sight meets, in the cells of a seat, and a side of a cell
/// there: one that is built, or the edge of the room there is to build in,
/// where the first cell of a build stands against it.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Aim {
    pub seat: Seat,
    /// The cell met, and the side the line came in through. On an edge of
    /// the room the cell is the one past it, so the cell before the side is
    /// the one a build starts in.
    pub hit: Hit,
    pub met: Met,
}

/// What a line of sight meets in the cells.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Met {
    /// A cell that is built.
    Cell,
    /// The ground of a volume: the cell before the side met holds it.
    Ground,
    /// A side of the room a hand builds in, seen from inside it: where the
    /// line leaves that box of cells.
    Side,
}

/// A line of sight bent into the cells of every seat built on, and what it
/// meets first.
pub struct Sight {
    paths: Vec<(Seat, Vec<[f64; 3]>)>,
    aim: Option<Aim>,
}

impl Sight {
    /// The nearest cell it meets, over any seat, unless the ground stands
    /// between it and where it starts: a cell a hill hides is not in sight.
    pub fn aim(&self) -> Option<Aim> {
        self.aim
    }

    /// Where it crosses the layer of a seat's cells across `axis` at `at`,
    /// whatever is behind, and over as many volumes as it reaches.
    pub fn crossing(&self, seat: impl Into<Seat>, axis: usize, at: f64) -> Option<[f64; 3]> {
        let seat = seat.into();
        let (_, path) = self.paths.iter().find(|(on, _)| *on == seat)?;
        crossing(path, axis, at)
    }
}

/// A change that landed, as what takes it back and puts it back.
enum Change {
    /// Cells changed: those of its box before and after.
    Cells {
        site: usize,
        span: Span,
        before: Vec<Cell>,
        after: Vec<Cell>,
    },
    /// A volume was closed: the cells it held, and every chunk of them that
    /// held something.
    Closed {
        seat: Seat,
        plot: [i32; 2],
        span: Span,
        chunks: Vec<([i32; 3], Vec<Cell>)>,
    },
}

/// A change made here and asked of the world, which has not answered: its
/// gestures, and the cells of its box as they were before it.
struct Pending {
    id: u32,
    site: usize,
    gestures: Vec<Gesture>,
    span: Span,
    before: Vec<Cell>,
}

/// What else was asked of the world and waits for its answer.
enum Waiting {
    Open(Seat, [i32; 2]),
    Close,
    TakeBack,
    PutBack,
}

/// An op to ask of the world's cells, in the envelope under their name.
pub(crate) struct Ask {
    pub kind: &'static str,
    pub payload: Vec<u8>,
    /// The number the world answers it by.
    pub id: u32,
}

/// The cells as a world keeps them, from where this client stands.
#[derive(Default)]
struct Link {
    /// This client's own session: a change it hears of under that number
    /// is one it made.
    me: u32,
    /// The last number given to an op.
    asked: u32,
    /// What to ask the world, in order.
    asks: Vec<Ask>,
    /// Changes made here that the world has not answered, the first first.
    pending: Vec<Pending>,
    waiting: HashMap<u32, Waiting>,
    /// The volumes held, each at the version the world last said of it.
    versions: BTreeMap<(Seat, [i32; 2]), u64>,
    /// The volumes held as they are seen from afar, each at the version it
    /// was seen at.
    afar: BTreeMap<(Seat, [i32; 2]), u64>,
    /// How many of this session's changes the world can take back, and put
    /// back.
    history: (usize, usize),
    since_look_s: f64,
}

impl Link {
    /// Queues an op for the world, under the next number.
    fn ask<M: Message>(&mut self, kind: &'static str, op: &M, waiting: Option<Waiting>) -> u32 {
        self.asked = self.asked.wrapping_add(1).max(1);
        let id = self.asked;
        self.asks.push(Ask {
            kind,
            payload: op.encode_to_vec(),
            id,
        });
        if let Some(waiting) = waiting {
            self.waiting.insert(id, waiting);
        }
        id
    }
}

/// Every volume of a world, what is drawn of them and what was changed.
#[derive(Default)]
pub struct Cells {
    /// The seats a volume stands on, in the order the first one opened.
    sites: Vec<Seated>,
    /// Where the centre of the moon is now, from the centre of the planet:
    /// what a place of a seat of the moon is counted from.
    moon: DVec3,
    /// What the ghost shows now: it is meshed again only when that changes.
    ghost: Option<(Seat, Gesture)>,
    /// The guides shown now, each under the id its place in the list gives.
    guides: Vec<Guide>,
    /// One more for every change to what the cells hold.
    revision: u64,
    /// The meshes of cubes drawn, each with the seat it is of.
    drawn: BTreeMap<VolumeMeshId, Seat>,
    /// The meshes of glass drawn, each beside the cubes of its chunk.
    glazed: BTreeMap<VolumeMeshId, Seat>,
    /// The light of each chunk that holds sides that shine, by its mesh,
    /// from the centre of the body its seat is on.
    lamps: BTreeMap<VolumeMeshId, (Seat, Lamp)>,
    /// Chunks owed a mesh since their cells changed, by site.
    stale: BTreeSet<(usize, [i32; 3])>,
    /// Volumes come near again, drawn as they are seen from afar until they
    /// are drawn whole.
    retiring: BTreeSet<(Seat, [i32; 2])>,
    /// Volumes left behind, drawn whole until they are drawn as they are
    /// seen from afar.
    leaving: BTreeSet<(Seat, [i32; 2])>,
    /// Whether a change of many gestures landed since the last update.
    landed: bool,
    changes: Vec<VolumeChange>,
    /// Where cells changed near enough the ground to cover or bare a tuft,
    /// as a cap of the body: its middle, unit, and its angle.
    touched: Vec<(DVec3, f64)>,
    /// Changes to take back, the last one last, and those taken back, while
    /// the cells are held here alone.
    done: Vec<Change>,
    undone: Vec<Change>,
    /// The world that keeps the cells, while this client is in one.
    link: Option<Link>,
}

impl Cells {
    /// The volumes of a seat, when something is built on it.
    fn site(&self, seat: Seat) -> Option<usize> {
        let whole = |site: &Seated| site.seat == seat && !site.afar();
        self.sites.iter().position(whole)
    }

    /// The volumes of a seat held as they are seen from afar, when some are.
    fn afar_site(&self, seat: Seat) -> Option<usize> {
        let afar = |site: &Seated| site.seat == seat && site.afar();
        self.sites.iter().position(afar)
    }

    /// Where the centre of a body is now.
    fn centre(&self, body: Body) -> DVec3 {
        match body {
            Body::Moon => self.moon,
            Body::Planet => DVec3::ZERO,
        }
    }

    /// The moon is somewhere else: what is seated on it goes with it.
    pub(crate) fn orbit(&mut self, moon: DVec3) {
        self.moon = moon;
    }

    /// A world keeps the cells from here on: what was held is forgotten, and
    /// what the world holds near the body is asked for at once.
    pub(crate) fn link(&mut self, me: u32) {
        self.clear();
        self.link = Some(Link {
            me,
            since_look_s: LOOK_S,
            ..Link::default()
        });
    }

    /// The world is gone, and what it kept with it.
    pub(crate) fn unlink(&mut self) {
        self.clear();
        self.link = None;
    }

    /// What to ask the world since the last call, in order.
    pub(crate) fn drain_asks(&mut self) -> Vec<Ask> {
        self.link
            .as_mut()
            .map(|link| core::mem::take(&mut link.asks))
            .unwrap_or_default()
    }

    /// How many times what the cells hold has changed: a volume opened or
    /// closed, a cell made, emptied or repainted. What was read of them at
    /// one count stands until the next.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Whether a volume stands anywhere on a body.
    pub fn stands_on(&self, body: Body) -> bool {
        let whole = |site: &Seated| site.seat.body() == body && !site.afar();
        self.sites.iter().any(whole)
    }

    /// Whether a volume stands on a column of a seat.
    pub fn covers(&self, seat: Seat, point: SurfacePoint) -> bool {
        self.site(seat)
            .is_some_and(|site| self.sites[site].volumes.is_open(plot_of(point)))
    }

    /// Whether the volume over a column of a seat is held as it is seen from
    /// afar.
    pub fn afar_over(&self, seat: Seat, point: SurfacePoint) -> bool {
        self.afar_site(seat)
            .is_some_and(|site| self.sites[site].volumes.is_open(plot_of(point)))
    }

    /// The cells the volume over a column of a seat holds.
    pub fn bounds_over(&self, seat: Seat, point: SurfacePoint) -> Option<Span> {
        self.sites[self.site(seat)?].volumes.bounds(plot_of(point))
    }

    /// Whether a volume holds a cell of a seat, air or not: where a gesture
    /// can land.
    pub fn holds(&self, seat: impl Into<Seat>, cell: [i32; 3]) -> bool {
        self.site(seat.into())
            .is_some_and(|site| self.sites[site].volumes.holds(cell))
    }

    /// What a cell of a seat is. Air where no volume holds it.
    pub fn cell(&self, seat: impl Into<Seat>, at: [i32; 3]) -> Cell {
        self.site(seat.into())
            .map_or_else(Cell::default, |site| self.sites[site].volumes.get(at))
    }

    /// Where a volume over the plot a column of a seat is on starts and
    /// ends, or why none can stand there.
    fn survey(
        &self,
        generator: &Generator,
        seat: Seat,
        point: SurfacePoint,
    ) -> Result<Stand, Refusal> {
        let field = matches!(generator.recipe().params.source, Source::Field(_));
        if self.link.is_some() && field {
            return Err(Refusal::Field);
        }
        seat::survey(generator, seat, point).map_err(|why| match why {
            Unseated::Sea => Refusal::Sea,
            Unseated::Seam => Refusal::Seam,
        })
    }

    /// The cells the volume of the plot a column of a seat is on holds, or
    /// would hold once opened: the room there is to build in. Why none can
    /// stand there, where none can.
    pub fn room(
        &self,
        generator: &Generator,
        seat: Seat,
        point: SurfacePoint,
    ) -> Result<Span, Refusal> {
        if let Some(held) = self.bounds_over(seat, point) {
            return Ok(held);
        }
        let stand = self.survey(generator, seat, point)?;
        let plot = plot_of(point);
        let mut opened = Volumes::new(PLOT_BITS);
        opened.open(plot, stand.low, stand.height);
        opened.bounds(plot).ok_or(Refusal::Seam)
    }

    /// Opens the volume of the plot a column of a seat is on, where none
    /// stands. The ground stays as it is, and the volume holds the blocks
    /// from the lowest of it to a height over the highest (`seat::survey`).
    /// In a world, the world is asked for it, and seats it by the same rule.
    pub(crate) fn open(
        &mut self,
        generator: &Generator,
        seat: Seat,
        point: SurfacePoint,
    ) -> Result<(), Refusal> {
        let stand = self.survey(generator, seat, point)?;
        if self.covers(seat, point) {
            return Ok(());
        }
        let plot = plot_of(point);
        self.stand(generator, seat, plot, stand);
        if let Some(link) = &mut self.link {
            let open = wire::Open {
                seat: Some(seat.wire()),
                u: point.u,
                v: point.v,
            };
            link.ask(wire::OPEN, &open, Some(Waiting::Open(seat, plot)));
            link.versions.insert((seat, plot), 0);
        }
        Ok(())
    }

    /// Stands a volume over a plot of a seat. Returns the seat's site.
    fn stand(&mut self, generator: &Generator, seat: Seat, plot: [i32; 2], stand: Stand) -> usize {
        let site = match self.site(seat) {
            Some(site) => site,
            None => {
                self.sites.push(Seated {
                    seat,
                    sphere: seat.sphere(generator),
                    scale: 1,
                    volumes: Volumes::new(PLOT_BITS),
                    balls: BTreeMap::new(),
                });
                self.sites.len() - 1
            }
        };
        self.sites[site].open(plot, stand.low, stand.height);
        self.revision += 1;
        site
    }

    /// Closes the volume over a column of a seat: what was built in it goes
    /// with it, and its plot is nature again. A change like any other, taken
    /// back as one. False where no volume stands. In a world, the world is
    /// asked to.
    pub(crate) fn close(&mut self, seat: Seat, point: SurfacePoint) -> bool {
        let plot = plot_of(point);
        let Some(span) = self.bounds_over(seat, point) else {
            return false;
        };
        match &mut self.link {
            Some(link) => {
                let close = wire::Close {
                    seat: Some(seat.wire()),
                    plot_x: plot[0],
                    plot_y: plot[1],
                };
                link.ask(wire::CLOSE, &close, Some(Waiting::Close));
            }
            None => {
                let held = self.site(seat).map(|site| {
                    let volumes = &self.sites[site].volumes;
                    let stored = volumes.stored(plot).into_iter();
                    stored
                        .map(|chunk| (chunk, volumes.cells(Volumes::chunk_span(chunk))))
                        .collect()
                });
                self.done.push(Change::Closed {
                    seat,
                    plot,
                    span,
                    chunks: held.unwrap_or_default(),
                });
                if self.done.len() > HISTORY {
                    self.done.remove(0);
                }
                self.undone.clear();
            }
        }
        self.remove(seat, plot);
        self.remove_afar(seat, plot);
        true
    }

    /// Takes away the volume over a plot, cells, picture and all.
    fn remove(&mut self, seat: Seat, plot: [i32; 2]) {
        self.leaving.remove(&(seat, plot));
        let Some(site) = self.site(seat) else {
            return;
        };
        let Some(bounds) = self.sites[site].volumes.bounds(plot) else {
            return;
        };
        for chunk in self.sites[site].volumes.chunks_in(bounds) {
            let id = self.sites[site].mesh_id(chunk);
            self.undraw(id);
            self.stale.remove(&(site, chunk));
        }
        let seated = &mut self.sites[site];
        seated.volumes.close(plot);
        // The grass under it grows again, and what stood against it shows
        // the sides it hid.
        if let Some((middle, _, across_m)) = seated.balls.remove(&plot)
            && seat.body() == Body::Planet
        {
            self.touched
                .push((middle.normalize(), across_m / middle.length()));
        }
        for chunk in seated.volumes.chunks_in(bounds.grown(1)) {
            self.stale.insert((site, chunk));
        }
        if let Some(link) = &mut self.link {
            link.versions.remove(&(seat, plot));
        }
        // The ghost showed what the cells were.
        if self.ghost.take().is_some() {
            self.changes.push(VolumeChange::Remove(GHOST));
        }
        self.revision += 1;
    }

    /// Takes the picture of a chunk away: its cubes, its glass and its lamp.
    fn undraw(&mut self, id: VolumeMeshId) {
        let glass = VolumeMeshId(id.0 | GLASS);
        if self.drawn.remove(&id).is_some() {
            self.changes.push(VolumeChange::Remove(id));
        }
        if self.glazed.remove(&glass).is_some() {
            self.changes.push(VolumeChange::Remove(glass));
        }
        self.lamps.remove(&id);
    }

    /// Forgets every volume: another world.
    pub(crate) fn clear(&mut self) {
        let drawn = core::mem::take(&mut self.drawn);
        for (id, _) in drawn.into_iter().chain(core::mem::take(&mut self.glazed)) {
            self.changes.push(VolumeChange::Remove(id));
        }
        self.lamps.clear();
        if self.ghost.take().is_some() {
            self.changes.push(VolumeChange::Remove(GHOST));
        }
        for slot in 0..core::mem::take(&mut self.guides).len() {
            self.changes.push(VolumeChange::Remove(guide_id(slot)));
        }
        self.revision += 1;
        self.sites.clear();
        self.stale.clear();
        self.retiring.clear();
        self.leaving.clear();
        self.landed = false;
        self.touched.clear();
        self.done.clear();
        self.undone.clear();
        if let Some(link) = &mut self.link {
            *link = Link {
                me: link.me,
                asked: link.asked,
                since_look_s: LOOK_S,
                ..Link::default()
            };
        }
    }

    /// Whether there is a change to take back, and one to put back. In a
    /// world, a change of this session's that the world can.
    pub fn history(&self) -> (bool, bool) {
        match &self.link {
            Some(link) => (link.history.0 > 0, link.history.1 > 0),
            None => (!self.done.is_empty(), !self.undone.is_empty()),
        }
    }

    /// Takes back the last change that landed. In a world, the world is
    /// asked to, and says what the cells are then.
    pub(crate) fn take_back(&mut self, generator: &Generator) {
        if let Some(link) = &mut self.link {
            if link.history.0 > 0 {
                link.ask(wire::TAKE_BACK, &wire::TakeBack {}, Some(Waiting::TakeBack));
            }
            return;
        }
        if let Some(change) = self.done.pop() {
            match &change {
                Change::Cells {
                    site, span, before, ..
                } => self.put(generator, *site, *span, before),
                // The volume stands again where it stood, as it was.
                Change::Closed {
                    seat,
                    plot,
                    span,
                    chunks,
                } => {
                    let stand = Stand {
                        low: span.min[2],
                        height: span.size()[2],
                    };
                    let site = self.stand(generator, *seat, *plot, stand);
                    for (chunk, cells) in chunks {
                        self.put(generator, site, Volumes::chunk_span(*chunk), cells);
                    }
                }
            }
            self.undone.push(change);
        }
    }

    /// Makes the cells of a box what they were at another time, and draws
    /// what that changed.
    fn put(&mut self, generator: &Generator, site: usize, span: Span, cells: &[Cell]) {
        if let Some(changed) = self.sites[site].volumes.restore(span, cells) {
            self.redraw(generator, site, changed);
        }
    }

    /// Puts back the last change taken back.
    pub(crate) fn put_back(&mut self, generator: &Generator) {
        if let Some(link) = &mut self.link {
            if link.history.1 > 0 {
                link.ask(wire::PUT_BACK, &wire::PutBack {}, Some(Waiting::PutBack));
            }
            return;
        }
        if let Some(change) = self.undone.pop() {
            match &change {
                Change::Cells {
                    site, span, after, ..
                } => self.put(generator, *site, *span, after),
                Change::Closed { seat, plot, .. } => self.remove(*seat, *plot),
            }
            self.done.push(change);
        }
    }

    /// One frame of the picture: meshes a few of the chunks owed one, the
    /// nearest the eye first. A change of many gestures is the work of the
    /// update it landed in, and its meshes start with the next.
    pub(crate) fn update(&mut self, eye: DVec3) {
        if !core::mem::take(&mut self.landed) {
            self.mesh_owed(eye, MESHES_PER_UPDATE);
        }
        self.retire();
    }

    /// Whether every chunk that changed is drawn as it is now.
    pub fn settled(&self) -> bool {
        self.stale.is_empty()
    }

    /// Meshes every chunk owed one: for a picture that must show all of it.
    pub(crate) fn settle(&mut self, eye: DVec3) {
        self.landed = false;
        self.mesh_owed(eye, usize::MAX);
        self.retire();
    }

    /// A line of sight from a point of the world along a direction, unit,
    /// and what it meets first: a cell, the ground of a volume, or a side of
    /// `room`, a box of a seat's cells, where the line leaves it. Ground no
    /// volume stands on hides what is behind it.
    pub fn sight(
        &self,
        generator: &Generator,
        from: DVec3,
        toward: DVec3,
        room: Option<(Seat, Span)>,
    ) -> Sight {
        // What is seen from afar is drawn, and nothing to aim at.
        let whole: Vec<&Seated> = self.sites.iter().filter(|site| !site.afar()).collect();
        let sights: Vec<(f64, Vec<[f64; 3]>)> = whole
            .iter()
            .map(|site| site.sight(from - self.centre(site.seat.body()), toward))
            .collect();
        let met = [Body::Planet, Body::Moon].into_iter().filter_map(|body| {
            let on = |site: &&Seated| site.seat.body() == body;
            let sites = whole
                .iter()
                .copied()
                .zip(&sights)
                .filter(|(site, _)| on(site));
            // How far the line could still reach a volume of this body.
            let far_m = sites
                .clone()
                .filter(|(_, (_, path))| !path.is_empty())
                .map(|(_, (far_m, _))| *far_m)
                .reduce(f64::max)?;
            let from = from - self.centre(body);
            // A cell built, and where the line leaves the room, whichever
            // comes first, over every seat.
            let met = sites
                .flat_map(|(site, (_, path))| {
                    let cell = site.volumes.trace(path).map(|hit| (hit, Met::Cell));
                    let side = room
                        .filter(|&(seat, _)| seat == site.seat)
                        .and_then(|(_, span)| voxel::leaves(span, path))
                        .map(|hit| (hit, Met::Side));
                    cell.into_iter().chain(side).map(|(hit, met)| {
                        let aim = Aim {
                            seat: site.seat,
                            hit,
                            met,
                        };
                        ((site.side(hit) - from).dot(toward), aim)
                    })
                })
                .min_by(|a, b| a.0.total_cmp(&b.0));
            // The ground comes first where it is nearer than the side met,
            // which may itself stand in the ground.
            let clear_m = met.map_or(far_m, |(away_m, _)| away_m - BLOCK_M / 2.0);
            match self.ground_met(generator, body, from, toward, clear_m) {
                Some(ground) => ground,
                None => met,
            }
        });
        let aim = met.min_by(|a, b| a.0.total_cmp(&b.0)).map(|(_, aim)| aim);
        let seats = whole.iter().map(|site| site.seat);
        Sight {
            paths: seats
                .zip(sights.into_iter().map(|(_, path)| path))
                .collect(),
            aim,
        }
    }

    /// Where a line of sight goes into the ground of a body within
    /// `clear_m` of where it starts, both said from the body's centre:
    /// `None` when it does not. Where it does, how far along, and the
    /// ground of a volume as what is aimed at; with no volume there, the
    /// ground hides what is behind it, and nothing is aimed at.
    fn ground_met(
        &self,
        generator: &Generator,
        body: Body,
        from: DVec3,
        toward: DVec3,
        clear_m: f64,
    ) -> Option<Option<(f64, Aim)>> {
        let radius_m = seat::sphere_of(generator, body).radius_m();
        let under = |along_m: f64| {
            let at = from + toward * along_m;
            let direction = at.normalize_or(DVec3::Y).to_array();
            at.length() - radius_m < seat::ground_m(generator, body, direction)
        };
        let steps = (clear_m / GROUND_STEP_M).floor().max(0.0) as usize;
        let step = (1..=steps).find(|&i| under(i as f64 * GROUND_STEP_M))?;
        let (mut over_m, mut under_m) = (
            (step - 1) as f64 * GROUND_STEP_M,
            step as f64 * GROUND_STEP_M,
        );
        for _ in 0..GROUND_HALVINGS {
            let middle_m = (over_m + under_m) / 2.0;
            match under(middle_m) {
                true => under_m = middle_m,
                false => over_m = middle_m,
            }
        }
        let at = from + toward * under_m;
        let sites = self.sites.iter().filter(|site| site.seat.body() == body);
        let aim = sites.filter_map(|site| {
            let cell = site.ground_cell(generator, at)?;
            let hit = Hit {
                cell: [cell[0], cell[1], cell[2] - 1],
                face: Face::UP,
            };
            Some(Aim {
                seat: site.seat,
                hit,
                met: Met::Ground,
            })
        });
        Some(aim.map(|aim| (under_m, aim)).next())
    }

    /// Where a lattice point of a seat's cells is in the world.
    pub fn corner(&self, generator: &Generator, seat: impl Into<Seat>, p: [i32; 3]) -> DVec3 {
        let seat = seat.into();
        let point = SurfacePoint::new(seat.sector(), f64::from(p[0]), f64::from(p[1]));
        let from_centre = seat
            .sphere(generator)
            .position(point, f64::from(p[2]) * BLOCK_M);
        self.centre(seat.body()) + DVec3::from(from_centre)
    }

    /// Applies gestures to the cells of a seat as one change, taken back as
    /// one, and draws what it changed. False when no cell changed. In a
    /// world the change is asked of the world too, and pending until it
    /// answers.
    pub(crate) fn apply(
        &mut self,
        generator: &Generator,
        seat: impl Into<Seat>,
        gestures: &[Gesture],
    ) -> bool {
        let seat = seat.into();
        let Some(site) = self.site(seat) else {
            return false;
        };
        let reach = gestures
            .iter()
            .map(|gesture| gesture.span())
            .reduce(|a, b| a.with(b.min).with(b.max));
        let volumes = &mut self.sites[site].volumes;
        // What is kept is what volumes hold of the change, however far it ran.
        let Some(span) = reach.and_then(|reach| volumes.held(reach)) else {
            return false;
        };
        // A box the world would not take is not made here either.
        if span.size().iter().map(|&n| u64::from(n)).product::<u64>() > seat::CHANGE_CELLS {
            return false;
        }
        let before = volumes.cells(span);
        let changed = gestures
            .iter()
            .filter_map(|&gesture| volumes.apply(gesture))
            .reduce(|a, b| a.with(b.min).with(b.max));
        let Some(changed) = changed else {
            return false;
        };
        match &mut self.link {
            Some(link) => {
                let change = wire::Change {
                    seat: Some(seat.wire()),
                    gestures: gestures
                        .iter()
                        .map(|&made| seat::gesture_wire(made))
                        .collect(),
                };
                let id = link.ask(wire::CHANGE, &change, None);
                link.pending.push(Pending {
                    id,
                    site,
                    gestures: gestures.to_vec(),
                    span,
                    before,
                });
            }
            None => {
                let after = volumes.cells(span);
                self.done.push(Change::Cells {
                    site,
                    span,
                    before,
                    after,
                });
                if self.done.len() > HISTORY {
                    self.done.remove(0);
                }
                self.undone.clear();
            }
        }
        self.landed |= gestures.len() > 1;
        self.redraw(generator, site, changed);
        true
    }

    /// Owes a mesh to every chunk whose sides a change to some cells could
    /// have changed: the cells, and one more all round, which is as far as a
    /// side or a corner looks. Where the change comes within a tuft of the
    /// ground the grass under it is to grow again.
    fn redraw(&mut self, generator: &Generator, site: usize, changed: Span) {
        self.revision += 1;
        // The ghost showed what the cells were: it is meshed again from what
        // they are now.
        if self.ghost.take().is_some() {
            self.changes.push(VolumeChange::Remove(GHOST));
        }
        let seated = &self.sites[site];
        let (low, high) = (changed.min, changed.max.map(|n| n + 1));
        let middle = [0, 1].map(|i| f64::from(low[i] + high[i]) / 2.0);
        let ends = [
            [low[0], low[1]],
            [high[0], low[1]],
            [low[0], high[1]],
            high[..2].try_into().expect("two of three"),
        ];
        // Grass grows on the planet alone.
        if seated.seat.body() == Body::Planet {
            let sector = seated.seat.sector();
            let highest = ends
                .iter()
                .map(|&[x, y]| [f64::from(x), f64::from(y)])
                .chain([middle])
                .map(|[u, v]| ground(generator, seated.seat, SurfacePoint::new(sector, u, v)))
                .fold(f64::MIN, f64::max);
            if f64::from(low[2]) <= highest + TUFT_M / BLOCK_M {
                let centre = seated.corner([middle[0] as i32, middle[1] as i32, low[2]]);
                let reach_m = seated.corner(low).distance(centre) + BLOCK_M;
                self.touched
                    .push((centre.normalize(), reach_m / centre.length()));
            }
        }
        for chunk in seated.volumes.chunks_in(changed.grown(1)) {
            self.stale.insert((site, chunk));
        }
    }

    /// Meshes up to `budget` of the chunks owed one, the nearest to `eye`
    /// first: its cubes, its glass apart from them, and the lamp its sides
    /// that shine come to. A chunk that holds nothing costs no mesh, and
    /// none of the budget.
    fn mesh_owed(&mut self, eye: DVec3, budget: usize) {
        if self.stale.is_empty() {
            return;
        }
        let half = CHUNK as i32 / 2;
        let mut owed: Vec<(f64, (usize, [i32; 3]))> = self
            .stale
            .iter()
            .map(|&(site, chunk)| {
                let seated = &self.sites[site];
                let middle = seated.corner(chunk.map(|n| n + half));
                let eye = eye - self.centre(seated.seat.body());
                (middle.distance_squared(eye), (site, chunk))
            })
            .collect();
        owed.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut left = budget;
        for (_, owed) in owed {
            if left == 0 {
                break;
            }
            self.stale.remove(&owed);
            let (site, chunk) = owed;
            if self.mesh_chunk(site, chunk) {
                left -= 1;
            }
        }
    }

    /// Meshes a chunk of a site: its cubes, its glass apart from them, and
    /// the lamp its sides that shine come to. False for a chunk that holds
    /// nothing, whose meshes go.
    fn mesh_chunk(&mut self, site: usize, chunk: [i32; 3]) -> bool {
        let id = self.sites[site].mesh_id(chunk);
        let quads = self.sites[site].volumes.faces(chunk);
        if quads.is_empty() {
            self.undraw(id);
            return false;
        }
        {
            let seated = &self.sites[site];
            let (seat, sphere, scale) = (seated.seat, seated.sphere, seated.scale);
            let sector = seat.sector();
            let glass = |quad: &Quad| Paint::of(quad.paint).finish == Finish::Glass;
            let (panes, cubes): (Vec<Quad>, Vec<Quad>) = quads.into_iter().partition(glass);
            // Afar, a light glows and lights nothing, and no side has an edge.
            let lit = match seated.afar() {
                true => None,
                false => lamp(sector, sphere, &cubes),
            };
            match lit {
                Some(lamp) => self.lamps.insert(id, (seat, lamp)),
                None => self.lamps.remove(&id),
            };
            let look = |paint: u8| {
                let (color, edge) = paint_look(paint);
                (color, if scale == 1 { edge } else { 0 })
            };
            let meshes = [
                (id, cubes, &mut self.drawn),
                (VolumeMeshId(id.0 | GLASS), panes, &mut self.glazed),
            ];
            for (id, quads, drawn) in meshes {
                if quads.is_empty() {
                    if drawn.remove(&id).is_some() {
                        self.changes.push(VolumeChange::Remove(id));
                    }
                    continue;
                }
                let mesh = mesh(sector, sphere, scale, &quads, look, 0.0);
                drawn.insert(id, seat);
                self.changes.push(VolumeChange::Add(id, mesh));
            }
        }
        true
    }

    /// Shows the ghost of a gesture, or of none: exactly the cells it would
    /// change, see-through, with a line between each cell and the next, so a
    /// hand sees what it will do and counts it. It is meshed again only when
    /// what it shows is another gesture, or the cells under it changed.
    pub(crate) fn preview(&mut self, gesture: Option<(Seat, Gesture)>) {
        let wanted = gesture.filter(|&(seat, _)| self.site(seat).is_some());
        if wanted == self.ghost {
            return;
        }
        self.ghost = wanted;
        let seated = wanted.and_then(|(seat, gesture)| Some((self.site(seat)?, gesture)));
        let Some((site, gesture)) = seated else {
            self.changes.push(VolumeChange::Remove(GHOST));
            return;
        };
        let seated = &self.sites[site];
        let rgb = match gesture {
            Gesture::Create { paint, .. } | Gesture::Paint { paint, .. } => paint_rgb(paint),
            Gesture::Delete { .. } => DELETE_COLOR,
        };
        let (color, ink) = ghost_look(rgb);
        let rim = [0; 4];
        let quads = seated.volumes.ghost(gesture);
        let sector = seated.seat.sector();
        let sides = mesh(sector, seated.sphere, 1, &quads, paint_look, GHOST_LIFT_M);
        // Each corner says where it is on its side, counted from a corner of
        // the frame every plot has, as the lines of a guide are.
        let plot = (1 << PLOT_BITS) - 1;
        let lattice = quads.iter().flat_map(|quad| {
            let (b, c) = ((quad.face.axis + 1) % 3, (quad.face.axis + 2) % 3);
            let from = [quad.cell[b] & !plot, quad.cell[c] & !plot];
            quad.corners()
                .map(|p| [(p[b] - from[0]) as f32, (p[c] - from[1]) as f32])
        });
        let corners = lattice.zip(&sides.vertices);
        let vertices = corners.map(|(lattice, corner)| GuideVertex {
            position: corner.position,
            lattice,
            color,
            ink,
            rim,
        });
        let mesh = GuideMesh {
            origin: sides.origin,
            vertices: vertices.collect(),
            indices: sides.indices,
        };
        self.changes.push(VolumeChange::Guide(GHOST, mesh));
    }

    /// Shows guides over the world, in place of those shown before: none,
    /// with none to show. A guide is meshed again only when it is another.
    pub(crate) fn guide(&mut self, generator: &Generator, wanted: &[Guide]) {
        if wanted == self.guides {
            return;
        }
        for slot in 0..wanted.len().max(self.guides.len()) {
            let id = guide_id(slot);
            match (wanted.get(slot), self.guides.get(slot)) {
                (Some(guide), shown) if shown != Some(guide) => {
                    let mesh = guide_mesh(guide.seat.sphere(generator), *guide);
                    self.changes.push(VolumeChange::Guide(id, mesh));
                }
                (None, Some(_)) => self.changes.push(VolumeChange::Remove(id)),
                _ => {}
            }
        }
        self.guides = wanted.to_vec();
    }

    /// What holds up a body standing at a column of a seat with its feet at
    /// `feet_m`, as far as the volumes go: `None` away from every volume.
    ///
    /// A body is as wide as [`BODY_HALF`] each way, so it stands on the
    /// highest cell under any part of it, over whichever volumes it
    /// straddles, and stops short of a wall.
    pub fn footing(&self, seat: Seat, point: SurfacePoint, feet_m: f64) -> Option<Footing> {
        let site = &self.sites[self.site(seat)?];
        let reach = (feet_m + STEP_M) / BLOCK_M;
        let columns = |at: f64| (at - BODY_HALF).floor() as i32..=(at + BODY_HALF).floor() as i32;
        let metres = |cells: i32| f64::from(cells) * BLOCK_M;
        columns(point.u)
            .flat_map(|x| columns(point.v).map(move |y| (x, y)))
            .filter_map(|(x, y)| site.volumes.gap(x, y, reach))
            .map(|gap| Footing {
                floor_m: gap.floor.map(metres),
                ceiling_m: gap.ceiling.map(metres),
            })
            .reduce(Footing::with)
    }

    /// Whether a cell stands in the way of something rooted at a column of a
    /// sector of the planet, `u` and `v` in blocks, between two heights in
    /// metres.
    pub fn covered(&self, sector: Sector, column: [f64; 2], heights: [f64; 2]) -> bool {
        let Some(site) = self.site(Seat::Sector(sector)) else {
            return false;
        };
        let [x, y] = column.map(|at| at.floor() as i32);
        let [from, to] = heights.map(|m| m / BLOCK_M);
        self.sites[site].volumes.covers(x, y, from, to)
    }

    /// Mesh uploads and removals since the last call, in order.
    pub(crate) fn drain_changes(&mut self) -> Vec<VolumeChange> {
        core::mem::take(&mut self.changes)
    }

    /// Where the grass is to grow again since the last call: caps of the
    /// body, each a middle, unit, and an angle.
    pub(crate) fn drain_touched(&mut self) -> Vec<(DVec3, f64)> {
        core::mem::take(&mut self.touched)
    }

    /// A mesh of a seat's cells as it is drawn now: around where the centre
    /// of the seat's body is.
    fn draw(&self, id: VolumeMeshId, seat: Seat) -> VolumeDraw {
        VolumeDraw {
            id,
            body_center: self.centre(seat.body()),
        }
    }

    /// Every mesh of cubes to draw.
    pub fn drawn(&self) -> Vec<VolumeDraw> {
        let drawn = self.drawn.iter();
        drawn.map(|(&id, &seat)| self.draw(id, seat)).collect()
    }

    /// Every mesh of glass to draw, over the cubes.
    pub fn glazed(&self) -> Vec<VolumeDraw> {
        let glazed = self.glazed.iter();
        glazed.map(|(&id, &seat)| self.draw(id, seat)).collect()
    }

    /// The lights among the cells held, each where it is in the world now.
    pub fn lamps(&self) -> Vec<Lamp> {
        let lamps = self.lamps.values().map(|&(seat, lamp)| Lamp {
            position: self.centre(seat.body()) + lamp.position,
            ..lamp
        });
        lamps.collect()
    }

    /// The ghost of the gesture a hand would make, while there is one.
    pub fn ghost(&self) -> Option<VolumeDraw> {
        self.ghost.map(|(seat, _)| self.draw(GHOST, seat))
    }

    /// The guides shown now.
    pub fn guides(&self) -> &[Guide] {
        &self.guides
    }

    /// The mesh of every guide to draw.
    pub fn guides_drawn(&self) -> Vec<VolumeDraw> {
        let guides = self.guides.iter().enumerate();
        guides
            .map(|(slot, guide)| self.draw(guide_id(slot), guide.seat))
            .collect()
    }
}

/// The cells as a world keeps them: what it says, and what it answers.
impl Cells {
    /// One frame of keeping in step with the world: once in a while, holds
    /// the volumes the body left behind as they are seen from afar, lets go
    /// of those it went far from, and asks the world what it holds near the
    /// body that this client lacks. `body` is the body of the world this
    /// client's own stands on or flies by, and where it is from the centre of
    /// it: what is seated on another body is far.
    pub(crate) fn look(&mut self, generator: &Generator, body: (Body, DVec3), dt: f64) {
        let Some(link) = &mut self.link else {
            return;
        };
        link.since_look_s += dt;
        if link.since_look_s < LOOK_S {
            return;
        }
        link.since_look_s = 0.0;
        let (on, from) = (body.0, body.1.to_array());
        // How far the body is from each volume held, whole or afar.
        let away: Vec<(bool, Seat, [i32; 2], f64)> = self
            .sites
            .iter()
            .flat_map(|site| {
                site.volumes.plots().filter_map(move |plot| {
                    let held = site.volumes.bounds(plot)?;
                    let [low, high] = [held.min[2], held.max[2] + 1].map(|n| n * site.scale);
                    let stand = Stand {
                        low,
                        height: (high - low) as u32,
                    };
                    let sector = site.seat.sector();
                    let away_m = match site.seat.body() == on {
                        true => seat::away_m(site.sphere, sector, plot, stand, from),
                        false => f64::INFINITY,
                    };
                    Some((site.afar(), site.seat, plot, away_m))
                })
            })
            .collect();
        for (afar, seat, plot, away_m) in away {
            let leaving = self.leaving.contains(&(seat, plot));
            match afar {
                true if away_m > AFAR_DROP_M => self.remove_afar(seat, plot),
                false if away_m > AFAR_DROP_M => {
                    self.remove(seat, plot);
                    self.remove_afar(seat, plot);
                }
                false if away_m > DROP_M && !leaving => self.leave(generator, seat, plot),
                // Come back before it was drawn afar: it stays whole.
                false if away_m <= DROP_M && leaving => {
                    self.leaving.remove(&(seat, plot));
                    self.remove_afar(seat, plot);
                }
                _ => {}
            }
        }
        let Some(link) = &mut self.link else {
            return;
        };
        let whole = link.versions.iter().map(|held| (held, false));
        let afar = link.afar.iter().map(|held| (held, true));
        let held = whole
            .chain(afar)
            .map(|((&(seat, plot), &version), afar)| wire::Held {
                seat: Some(seat.wire()),
                plot_x: plot[0],
                plot_y: plot[1],
                version,
                afar,
            });
        let look = wire::Look {
            held: held.collect(),
        };
        link.ask(wire::LOOK, &look, None);
    }

    /// An event of the world's cells.
    pub(crate) fn receive(&mut self, generator: &Generator, kind: &str, payload: &[u8]) {
        let Some(me) = self.link.as_ref().map(|link| link.me) else {
            return;
        };
        match kind {
            wire::OPENED => {
                if let Ok(opened) = wire::Opened::decode(payload) {
                    self.opened(generator, &opened);
                }
            }
            wire::CHANGED => {
                if let Ok(changed) = wire::Changed::decode(payload) {
                    self.changed(generator, &changed, me);
                }
            }
            wire::RESTORED => {
                if let Ok(restored) = wire::Restored::decode(payload) {
                    self.restored(generator, &restored);
                }
            }
            wire::SEEN => {
                if let Ok(seen) = wire::Seen::decode(payload) {
                    self.seen(generator, seen);
                }
            }
            _ => {}
        }
    }

    /// Someone opened a volume near: it stands here too.
    fn opened(&mut self, generator: &Generator, opened: &wire::Opened) {
        let seat = Seat::from_wire(opened.seat.as_ref());
        let (Some(seat), Some(stood)) = (seat, &opened.stood) else {
            return;
        };
        let plot = [stood.plot_x, stood.plot_y];
        let stand = Stand {
            low: stood.low,
            height: stood.height,
        };
        self.stand(generator, seat, plot, stand);
        if let Some(link) = &mut self.link {
            link.versions.entry((seat, plot)).or_insert(stood.version);
        }
    }

    /// Gestures landed. Someone else's are made here, under what this client
    /// made and the world has not answered; its own are here already.
    fn changed(&mut self, generator: &Generator, changed: &wire::Changed, me: u32) {
        let Some(seat) = Seat::from_wire(changed.seat.as_ref()) else {
            return;
        };
        let gestures: Option<Vec<Gesture>> = changed
            .gestures
            .iter()
            .map(seat::gesture_from_wire)
            .collect();
        if let (Some(site), Some(gestures), true) =
            (self.site(seat), gestures, changed.session != me)
        {
            self.rebase(generator, site, None, |volumes| {
                let made = gestures.iter().filter_map(|&made| volumes.apply(made));
                made.reduce(|a, b| a.with(b.min).with(b.max))
            });
        }
        self.counted(seat, &changed.stood);
    }

    /// A change was taken back or put back: the cells of its box are as the
    /// world says, under what this client made and the world has not
    /// answered.
    fn restored(&mut self, generator: &Generator, restored: &wire::Restored) {
        let Some(seat) = Seat::from_wire(restored.seat.as_ref()) else {
            return;
        };
        let span = Span::between(
            [restored.x0, restored.y0, restored.z0],
            [restored.x1, restored.y1, restored.z1],
        );
        // A box no volume here could hold is no box of this world's.
        let count: u64 = span.size().iter().map(|&n| u64::from(n)).product();
        let cells = (count <= 1 << 24).then(|| unpack(&restored.cells, count as usize));
        if let (Some(site), Some(Some(cells))) = (self.site(seat), cells) {
            self.rebase(generator, site, None, |volumes| {
                volumes.restore(span, &cells)
            });
        }
        self.counted(seat, &restored.stood);
    }

    /// The world's answer to a look: volumes whole, and those that are gone.
    fn seen(&mut self, generator: &Generator, seen: wire::Seen) {
        for volume in seen.volumes {
            let seat = Seat::from_wire(volume.seat.as_ref());
            let (Some(seat), Some(stood)) = (seat, volume.stood) else {
                continue;
            };
            let plot = [stood.plot_x, stood.plot_y];
            let stand = Stand {
                low: stood.low,
                height: stood.height,
            };
            // As it is seen from afar: held so, and whole no more.
            if volume.afar {
                let chunks: Vec<([i32; 3], Vec<Cell>)> = volume
                    .chunks
                    .iter()
                    .filter_map(|chunk| {
                        let cells = unpack(&chunk.cells, CHUNK_AFAR_CELLS)?;
                        Some(([chunk.x, chunk.y, chunk.z], cells))
                    })
                    .collect();
                self.remove(seat, plot);
                self.stand_afar(generator, seat, plot, stand, &chunks);
                if let Some(link) = &mut self.link {
                    link.afar.insert((seat, plot), stood.version);
                }
                continue;
            }
            // Held afar, it is drawn so until it is drawn whole.
            if self
                .afar_site(seat)
                .is_some_and(|site| self.sites[site].volumes.is_open(plot))
            {
                self.retiring.insert((seat, plot));
            }
            // Held at the version shown, this is more of the same volume.
            // At another, or not at all, it is the volume anew.
            let held = self
                .link
                .as_ref()
                .and_then(|link| link.versions.get(&(seat, plot)));
            if held != Some(&stood.version) {
                self.remove(seat, plot);
            }
            let site = self.stand(generator, seat, plot, stand);
            let Some(bounds) = self.sites[site].volumes.bounds(plot) else {
                continue;
            };
            if let Some(link) = &mut self.link {
                link.versions.insert((seat, plot), stood.version);
            }
            // What is drawn again is what the chunks shown changed: a volume
            // is mostly air, and air owes no mesh.
            self.rebase(generator, site, Some(bounds), |volumes| {
                let mut changed: Option<Span> = None;
                for chunk in &volume.chunks {
                    if let Some(cells) = unpack(&chunk.cells, CHUNK_CELLS) {
                        let span = Volumes::chunk_span([chunk.x, chunk.y, chunk.z]);
                        if let Some(made) = volumes.restore(span, &cells) {
                            let so_far = changed.unwrap_or(made);
                            changed = Some(so_far.with(made.min).with(made.max));
                        }
                    }
                }
                changed
            });
        }
        for gone in seen.gone {
            if let Some(seat) = Seat::from_wire(gone.seat.as_ref()) {
                self.remove(seat, [gone.plot_x, gone.plot_y]);
                self.remove_afar(seat, [gone.plot_x, gone.plot_y]);
            }
        }
    }

    /// Stands the volume over a plot of a seat as it is seen from afar, anew:
    /// the cells afar of each chunk it holds, by the chunk's lowest corner in
    /// the seat's cells. What was drawn of it stays until it is drawn again.
    fn stand_afar(
        &mut self,
        generator: &Generator,
        seat: Seat,
        plot: [i32; 2],
        stand: Stand,
        chunks: &[([i32; 3], Vec<Cell>)],
    ) -> usize {
        let site = match self.afar_site(seat) {
            Some(site) => site,
            None => {
                self.sites.push(Seated {
                    seat,
                    sphere: seat.sphere(generator),
                    scale: AFAR,
                    volumes: Volumes::new(PLOT_BITS - AFAR.trailing_zeros()),
                    balls: BTreeMap::new(),
                });
                self.sites.len() - 1
            }
        };
        let volumes = &mut self.sites[site].volumes;
        let before = volumes.bounds(plot).map(|bounds| volumes.chunks_in(bounds));
        volumes.close(plot);
        let low = stand.low.div_euclid(AFAR);
        let top = (stand.low + stand.height as i32 + AFAR - 1).div_euclid(AFAR);
        volumes.open(plot, low, (top - low) as u32);
        for (corner, cells) in chunks {
            let at = corner.map(|n| n.div_euclid(AFAR));
            volumes.restore(Span::between(at, at.map(|n| n + CHUNK_AFAR - 1)), cells);
        }
        let now = volumes.bounds(plot).map(|bounds| volumes.chunks_in(bounds));
        for chunk in before.into_iter().chain(now).flatten() {
            self.stale.insert((site, chunk));
        }
        site
    }

    /// Holds a volume the body left behind as it is seen from afar, made
    /// from what is held of it. It is drawn whole until it is drawn afar, a
    /// few chunks an update, so letting it go costs no frame more than
    /// drawing any other change.
    fn leave(&mut self, generator: &Generator, seat: Seat, plot: [i32; 2]) {
        let Some(site) = self.site(seat) else {
            return;
        };
        let volumes = &self.sites[site].volumes;
        let Some(bounds) = volumes.bounds(plot) else {
            return;
        };
        let chunks: Vec<([i32; 3], Vec<Cell>)> = volumes
            .stored(plot)
            .into_iter()
            .map(|chunk| {
                (
                    chunk,
                    voxel::afar(&volumes.cells(Volumes::chunk_span(chunk))),
                )
            })
            .collect();
        let stand = Stand {
            low: bounds.min[2],
            height: bounds.size()[2],
        };
        self.stand_afar(generator, seat, plot, stand, &chunks);
        self.leaving.insert((seat, plot));
        if let Some(link) = &mut self.link
            && let Some(&version) = link.versions.get(&(seat, plot))
        {
            link.afar.insert((seat, plot), version);
        }
    }

    /// Takes away a volume held as it is seen from afar, and its picture.
    fn remove_afar(&mut self, seat: Seat, plot: [i32; 2]) {
        self.retiring.remove(&(seat, plot));
        self.leaving.remove(&(seat, plot));
        if let Some(link) = &mut self.link {
            link.afar.remove(&(seat, plot));
        }
        let Some(site) = self.afar_site(seat) else {
            return;
        };
        let Some(bounds) = self.sites[site].volumes.bounds(plot) else {
            return;
        };
        for chunk in self.sites[site].volumes.chunks_in(bounds) {
            let id = self.sites[site].mesh_id(chunk);
            self.undraw(id);
            self.stale.remove(&(site, chunk));
        }
        self.sites[site].volumes.close(plot);
    }

    /// Lets go of what is drawn afar of the volumes come near, once they are
    /// drawn whole, and of what is drawn whole of the volumes left behind,
    /// once they are drawn afar.
    fn retire(&mut self) {
        let drawn = |cells: &Cells, site: Option<usize>, plot: [i32; 2]| {
            let Some(site) = site else {
                return true;
            };
            let volumes = &cells.sites[site].volumes;
            !cells
                .stale
                .iter()
                .any(|&(at, chunk)| at == site && volumes.plot_of(chunk[0], chunk[1]) == plot)
        };
        let near: Vec<(Seat, [i32; 2])> = self
            .retiring
            .iter()
            .copied()
            .filter(|&(seat, plot)| drawn(self, self.site(seat), plot))
            .collect();
        for (seat, plot) in near {
            self.remove_afar(seat, plot);
        }
        let far: Vec<(Seat, [i32; 2])> = self
            .leaving
            .iter()
            .copied()
            .filter(|&(seat, plot)| drawn(self, self.afar_site(seat), plot))
            .collect();
        for (seat, plot) in far {
            self.leaving.remove(&(seat, plot));
            self.remove(seat, plot);
        }
    }

    /// Counts a change in the volumes it landed in. A volume that missed one
    /// stays at the version it had, and the next look brings it whole.
    fn counted(&mut self, seat: Seat, stood: &[wire::Stood]) {
        let Some(link) = &mut self.link else {
            return;
        };
        for stood in stood {
            if let Some(held) = link.versions.get_mut(&(seat, [stood.plot_x, stood.plot_y]))
                && *held + 1 == stood.version
            {
                *held = stood.version;
            }
        }
    }

    /// Changes the cells of a site as the world says, under what this client
    /// made and the world has not answered: its pending changes come off,
    /// the world's change is made, and they go back on, each over what is
    /// there now. With `fresh`, the box of a volume the world just showed
    /// whole, there is nothing of the pending under it to take off.
    fn rebase(
        &mut self,
        generator: &Generator,
        site: usize,
        fresh: Option<Span>,
        change: impl FnOnce(&mut Volumes) -> Option<Span>,
    ) {
        let join = |a: Option<Span>, b: Option<Span>| match (a, b) {
            (Some(a), Some(b)) => Some(a.with(b.min).with(b.max)),
            (a, b) => a.or(b),
        };
        let mut pending = match &mut self.link {
            Some(link) => core::mem::take(&mut link.pending),
            None => Vec::new(),
        };
        let volumes = &mut self.sites[site].volumes;
        let mut changed = None;
        if fresh.is_none() {
            for made in pending.iter().rev().filter(|made| made.site == site) {
                changed = join(changed, volumes.restore(made.span, &made.before));
            }
        }
        changed = join(changed, change(volumes));
        for made in pending.iter_mut().filter(|made| made.site == site) {
            made.before = volumes.cells(made.span);
            for &gesture in &made.gestures {
                changed = join(changed, volumes.apply(gesture));
            }
        }
        if let Some(link) = &mut self.link {
            link.pending = pending;
        }
        if let Some(changed) = changed {
            self.redraw(generator, site, changed);
        }
    }

    /// Takes a refused change off from under what was made after it: those
    /// come off first, it comes off, and they go back on.
    fn unmake(&mut self, generator: &Generator, refused: Pending) {
        let Some(link) = &mut self.link else {
            return;
        };
        // What was made before it stays as it is: only what came after it
        // may stand on what it made.
        let later = link
            .pending
            .iter()
            .position(|made| made.id.wrapping_sub(refused.id) < u32::MAX / 2)
            .unwrap_or(link.pending.len());
        let earlier: Vec<Pending> = link.pending.drain(..later).collect();
        self.rebase(generator, refused.site, None, |volumes| {
            volumes.restore(refused.span, &refused.before)
        });
        if let Some(link) = &mut self.link {
            link.pending.splice(0..0, earlier);
        }
    }

    /// The world's answer to an op of the cells: it landed, with an empty
    /// code, or the code of why it was refused. False for an answer to
    /// nothing the cells asked.
    pub(crate) fn answered(&mut self, generator: &Generator, id: u32, code: &str) -> bool {
        let Some(link) = &mut self.link else {
            return false;
        };
        let landed = code.is_empty();
        if let Some(at) = link.pending.iter().position(|made| made.id == id) {
            if landed {
                link.pending.remove(at);
                link.history = (link.history.0 + 1, 0);
                return true;
            }
            // Refused: it comes off from under what was made after it.
            let refused = link.pending.remove(at);
            self.unmake(generator, refused);
            return true;
        }
        match link.waiting.remove(&id) {
            Some(Waiting::Open(seat, plot)) => {
                if !landed {
                    self.remove(seat, plot);
                }
            }
            // A volume the world kept is asked for again at once.
            Some(Waiting::Close) => match landed {
                true => link.history = (link.history.0 + 1, 0),
                false => link.since_look_s = LOOK_S,
            },
            Some(Waiting::TakeBack) => {
                link.history = match landed {
                    true => (link.history.0.saturating_sub(1), link.history.1 + 1),
                    false => (0, link.history.1),
                };
            }
            Some(Waiting::PutBack) => {
                link.history = match landed {
                    true => (link.history.0 + 1, link.history.1.saturating_sub(1)),
                    false => (link.history.0, 0),
                };
            }
            None => return false,
        }
        true
    }
}

/// A colour of the palette for each a paint can be.
const _: () = assert!(PALETTE.len() == Paint::COLORS as usize);

/// The colour of a paint.
fn paint_rgb(paint: u8) -> [u8; 3] {
    PALETTE[usize::from(Paint::of(paint).color)]
}

/// How the sides of a paint are drawn: its colour and how much it shines
/// with it, then its edge.
fn paint_look(paint: u8) -> ([u8; 4], u8) {
    let [r, g, b] = paint_rgb(paint);
    let paint = Paint::of(paint);
    let shine = match paint.finish {
        Finish::Light => u8::MAX,
        Finish::Matte | Finish::Glass => 0,
    };
    let edge = match paint.edge {
        Edge::None => 0,
        Edge::Black => 1,
        Edge::White => 2,
    };
    ([r, g, b, shine], edge)
}

/// How what a hand would make is drawn in a colour: the colour between the
/// lines, and lines that stand out of it, black over a pale colour and
/// white over a dark one.
fn ghost_look([r, g, b]: [u8; 3]) -> ([u8; 4], [u8; 4]) {
    let pale = u32::from(r) * 3 + u32::from(g) * 6 + u32::from(b) > 1400;
    let ink = if pale { 0 } else { u8::MAX };
    ([r, g, b, GHOST_FILL], [ink, ink, ink, GHOST_INK])
}

/// The lamp the sides that shine among some come to: where their middle is,
/// their colour, and brighter and further reaching the more they are.
fn lamp(sector: Sector, sphere: QuadSphere, quads: &[Quad]) -> Option<Lamp> {
    let (mut sides, mut middle, mut color) = (0.0_f32, DVec3::ZERO, Vec3::ZERO);
    for quad in quads {
        if Paint::of(quad.paint).finish != Finish::Light {
            continue;
        }
        sides += 1.0;
        let [p, _, q, _] = quad.corners();
        middle += (DVec3::from(p.map(f64::from)) + DVec3::from(q.map(f64::from))) / 2.0;
        color += Vec3::from(paint_rgb(quad.paint).map(|c| (f32::from(c) / 255.0).powf(2.2)));
    }
    if sides == 0.0 {
        return None;
    }
    let middle = middle / f64::from(sides);
    let point = SurfacePoint::new(sector, middle.x, middle.y);
    let many = sides.min(LAMP_SIDES).sqrt();
    Some(Lamp {
        position: DVec3::from(sphere.position(point, middle.z * BLOCK_M)),
        color: color / sides * LAMP_GAIN * many,
        reach_m: (LAMP_REACH_M + LAMP_GROWS_M * (many - 1.0)).min(LAMP_FAR_M),
    })
}

fn guide_id(slot: usize) -> VolumeMeshId {
    VolumeMeshId(GUIDES - slot as u64)
}

/// The sides of a guide's box, bent onto the body its seat is on as the
/// cells are and standing a little off it, each corner saying where it is on
/// its side.
fn guide_mesh(sphere: QuadSphere, guide: Guide) -> GuideMesh {
    let (low, high) = (guide.span.min, guide.span.max.map(|n| n + 1));
    let mut mesh = GuideMesh {
        origin: DVec3::ZERO,
        vertices: Vec::new(),
        indices: Vec::new(),
    };
    let sector = guide.seat.sector();
    let (color, ink, rim) = match guide.paint {
        Some(paint) => {
            let (color, ink) = ghost_look(paint_rgb(paint));
            (color, ink, [0; 4])
        }
        None => {
            let [r, g, b] = ROOM_COLOR;
            ([r, g, b, ROOM_FILL], [r, g, b, ROOM_INK], ROOM_RIM)
        }
    };
    let at = |p: [f64; 3]| {
        let point = SurfacePoint::new(sector, p[0], p[1]);
        DVec3::from(sphere.position(point, p[2] * BLOCK_M))
    };
    mesh.origin = at(low.map(f64::from));
    // Lines are counted from a corner of the frame every plot has: those of
    // two guides side by side meet.
    let from = |n: i32| (n.rem_euclid(1 << PLOT_BITS)) as f32;
    // Where the quads of a side are cut along an axis: every few cells
    // along the ground, which curves, and nowhere up, which is straight.
    let cuts = |axis: usize| -> Vec<i32> {
        let step = match axis {
            2 => usize::MAX,
            _ => GUIDE_STEP as usize,
        };
        let mut cuts: Vec<i32> = (low[axis]..high[axis]).step_by(step).collect();
        cuts.push(high[axis]);
        cuts
    };
    for axis in 0..3 {
        let (a, b) = ((axis + 1) % 3, (axis + 2) % 3);
        let (along_a, along_b) = (cuts(a), cuts(b));
        for (level, out) in [(low[axis], -GUIDE_LIFT), (high[axis], GUIDE_LIFT)] {
            for across_a in along_a.windows(2) {
                for across_b in along_b.windows(2) {
                    let base = mesh.vertices.len() as u32;
                    let corners = [
                        (across_a[0], across_b[0]),
                        (across_a[1], across_b[0]),
                        (across_a[1], across_b[1]),
                        (across_a[0], across_b[1]),
                    ];
                    for (pa, pb) in corners {
                        let mut p = [0.0; 3];
                        p[axis] = f64::from(level) + out;
                        p[a] = f64::from(pa);
                        p[b] = f64::from(pb);
                        mesh.vertices.push(GuideVertex {
                            position: (at(p) - mesh.origin).as_vec3().to_array(),
                            lattice: [
                                from(low[a]) + (pa - low[a]) as f32,
                                from(low[b]) + (pb - low[b]) as f32,
                            ],
                            color,
                            ink,
                            rim,
                        });
                    }
                    mesh.indices
                        .extend([0, 1, 2, 0, 2, 3].map(|corner| base + corner));
                }
            }
        }
    }
    mesh
}

/// Bends the sides of a sector's cells onto the planet: every corner goes
/// through the address it is, so neighbouring sides share their corners
/// exactly, across two volumes as within one, and a volume on a small world
/// curves with it. `lift_m` stands each side off along its own normal, and
/// `look` says how the sides of a paint are drawn: their colour and shine,
/// then their edge.
fn mesh(
    sector: Sector,
    sphere: QuadSphere,
    scale: i32,
    quads: &[Quad],
    look: impl Fn(u8) -> ([u8; 4], u8),
    lift_m: f32,
) -> VolumeMesh {
    let corners = |quad: &Quad| quad.corners().map(|p| p.map(|n| n * scale));
    let Some(first) = quads.first() else {
        return VolumeMesh {
            origin: DVec3::ZERO,
            vertices: Vec::new(),
            indices: Vec::new(),
        };
    };
    // The directions of every column corner the sides use, worked out once:
    // a direction costs two tangents, a height only a multiply.
    let (mut low, mut high) = ([i32::MAX; 2], [i32::MIN; 2]);
    for corner in quads.iter().flat_map(corners) {
        for i in 0..2 {
            low[i] = low[i].min(corner[i]);
            high[i] = high[i].max(corner[i]);
        }
    }
    let width = (high[0] - low[0] + 1) as usize;
    let directions: Vec<DVec3> = (low[1]..=high[1])
        .flat_map(|y| (low[0]..=high[0]).map(move |x| (x, y)))
        .map(|(x, y)| {
            let point = SurfacePoint::new(sector, f64::from(x), f64::from(y));
            DVec3::from(sphere.blocks().direction(point))
        })
        .collect();
    let radius_m = sphere.radius_m();
    let at = |p: [i32; 3]| {
        let direction = directions[(p[1] - low[1]) as usize * width + (p[0] - low[0]) as usize];
        direction * (radius_m + f64::from(p[2]) * BLOCK_M)
    };
    let base_at = at(corners(first)[0]);

    let mut vertices = Vec::with_capacity(quads.len() * 4);
    let mut indices = Vec::with_capacity(quads.len() * 6);
    for quad in quads {
        let corners = corners(quad).map(|p| (at(p) - base_at).as_vec3());
        // Each sector's frame is right handed (`u x v` is out), so a side
        // wound counter clockwise in cells is wound so in the world.
        let normal = (corners[1] - corners[0])
            .cross(corners[3] - corners[0])
            .normalize_or(Vec3::Z);
        let (color, edge) = look(quad.paint);
        let base = vertices.len() as u32;
        // Round the side from its first corner: across, then up, then back.
        let across = [[0, 0], [u8::MAX, 0], [u8::MAX, u8::MAX], [0, u8::MAX]];
        for ((corner, open), [u, v]) in corners.into_iter().zip(quad.open).zip(across) {
            vertices.push(VolumeVertex {
                position: (corner + normal * lift_m).to_array(),
                normal: normal.to_array(),
                color,
                side: [open * (u8::MAX / 3), u, v, edge],
            });
        }
        let order = if quad.flipped() {
            [1, 2, 3, 1, 3, 0]
        } else {
            [0, 1, 2, 0, 2, 3]
        };
        indices.extend(order.map(|k| base + k));
    }
    VolumeMesh {
        origin: base_at,
        vertices,
        indices,
    }
}

#[cfg(test)]
impl Cells {
    /// Opens the volume over a column and lays a floor across its plot, one
    /// cell thick, over the highest ground under it. Returns the first cell
    /// over the corner of the floor: tests speak in cells counted from there.
    pub(crate) fn floor(&mut self, generator: &Generator, point: SurfacePoint) -> [i32; 3] {
        self.open(generator, point.sector.into(), point)
            .expect("dry land");
        let [x, y] = plot_of(point).map(|n| n << PLOT_BITS);
        let side = 1 << PLOT_BITS;
        let highest = (0..=side)
            .flat_map(|dy| (0..=side).map(move |dx| (dx, dy)))
            .map(|(dx, dy)| {
                let (u, v) = (f64::from(x + dx), f64::from(y + dy));
                ground(
                    generator,
                    point.sector.into(),
                    SurfacePoint::new(point.sector, u, v),
                )
            })
            .fold(f64::MIN, f64::max);
        let top = highest.ceil() as i32;
        let far = side - 1;
        let span = Span::between([x, y, top - 1], [x + far, y + far, top - 1]);
        self.apply(
            generator,
            point.sector,
            &[Gesture::Create { span, paint: 0 }],
        );
        // The floor is there to stand on, not to take back.
        self.done.clear();
        self.changes.clear();
        [x, y, top]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use worldgen::Recipe;

    fn world() -> (Generator, SurfacePoint) {
        let generator = Generator::new(Recipe::new(1)).unwrap();
        let side = f64::from(generator.sphere().blocks().side());
        let sector = Sector::new(4).unwrap();
        (
            generator,
            SurfacePoint::new(sector, side * 0.41, side * 0.37),
        )
    }

    /// A world with a floor laid across the plot where [`world`] stands, and
    /// the first cell over its corner.
    fn opened() -> (Generator, Cells, [i32; 3]) {
        let (generator, point) = world();
        let mut cells = Cells::default();
        let origin = cells.floor(&generator, point);
        (generator, cells, origin)
    }

    /// A column `plots` plots along `u` from another.
    fn along(point: SurfacePoint, plots: i32) -> SurfacePoint {
        let blocks = f64::from(plots << PLOT_BITS);
        SurfacePoint::new(point.sector, point.u + blocks, point.v)
    }

    fn at(origin: [i32; 3], cell: [i32; 3]) -> [i32; 3] {
        [0, 1, 2].map(|i| origin[i] + cell[i])
    }

    fn create(a: [i32; 3], b: [i32; 3], paint: u8) -> Gesture {
        Gesture::Create {
            span: Span::between(a, b),
            paint,
        }
    }

    /// The middle of the top of a cell.
    fn top(cells: &Cells, cell: [i32; 3]) -> DVec3 {
        let site = &cells.sites[0];
        let low = site.corner([cell[0], cell[1], cell[2] + 1]);
        let high = site.corner([cell[0] + 1, cell[1] + 1, cell[2] + 1]);
        (low + high) / 2.0
    }

    /// Every cell over the floor that is not air, in the volume the tests
    /// open, counted from the first: as high as the tests build.
    fn laid(cells: &Cells, origin: [i32; 3]) -> Vec<[i32; 3]> {
        let volumes = &cells.sites[0].volumes;
        let plot = volumes.plot_of(origin[0], origin[1]);
        let mut over = volumes.bounds(plot).expect("an open volume");
        over.min[2] = origin[2];
        over.max[2] = over.max[2].min(origin[2] + 64);
        over.cells()
            .filter(|&cell| !volumes.get(cell).is_air())
            .map(|cell| [0, 1, 2].map(|i| cell[i] - origin[i]))
            .collect()
    }

    #[test]
    fn a_volume_opens_over_the_plot_under_the_body_and_leaves_the_ground() {
        let (generator, point) = world();
        let direction = generator.sphere().blocks().direction(point);
        let ground_m = generator.sample(direction).height_m;
        let mut cells = Cells::default();
        cells
            .open(&generator, point.sector.into(), point)
            .expect("dry land");
        assert!(cells.covers(point.sector.into(), point));
        assert!(!cells.covers(point.sector.into(), along(point, 1)));
        // The address cuts it, wherever on the plot the body stood, and it
        // holds the ground of the plot and the air over it.
        let held = cells.bounds_over(point.sector.into(), point).unwrap();
        assert_eq!(held.min[0] % 64, 0);
        assert_eq!(held.min[1] % 64, 0);
        assert_eq!(held.size()[0], 64);
        assert_eq!(held.min[2] % 16, 0);
        let feet = (ground_m / BLOCK_M) as i32;
        assert!(held.contains([point.u as i32, point.v as i32, feet]));
        assert!(held.max[2] >= feet + seat::HEIGHT);
        // Nature is as it was: no stamp, and nothing built.
        assert!(generator.stamps().is_empty());
        assert_eq!(generator.sample(direction).height_m, ground_m);
        assert!(cells.drain_changes().is_empty());
    }

    #[test]
    fn a_plot_on_the_edge_of_a_sector_stays_nature() {
        let (generator, point) = world();
        let side = f64::from(generator.sphere().blocks().side());
        let mut cells = Cells::default();
        for (u, v) in [(10.0, point.v), (point.u, side - 10.0), (side - 1.0, 1.0)] {
            let edge = SurfacePoint::new(point.sector, u, v);
            assert_eq!(
                cells.open(&generator, edge.sector.into(), edge),
                Err(Refusal::Seam),
                "{u} {v}"
            );
        }
        // One plot in, the edge is no reason: the sea may be.
        let inside = SurfacePoint::new(point.sector, 70.0, side - 70.0);
        assert_ne!(
            cells.open(&generator, inside.sector.into(), inside),
            Err(Refusal::Seam)
        );
    }

    #[test]
    fn a_change_of_many_chunks_is_drawn_over_updates_the_nearest_first() {
        let (generator, mut cells, origin) = opened();
        let sector = cells.sites[0].seat.sector();
        cells.settle(DVec3::ZERO);
        cells.drain_changes();
        // Two blocks the width of the plot, sixteen cells high.
        let blocks = [
            create(at(origin, [0, 0, 0]), at(origin, [63, 31, 15]), 1),
            create(at(origin, [0, 32, 0]), at(origin, [63, 63, 15]), 2),
        ];
        assert!(cells.apply(&generator, sector, &blocks));
        let owed = cells.stale.len();
        assert!(owed > MESHES_PER_UPDATE, "{owed}");
        let eye = top(&cells, at(origin, [20, 20, 20]));
        let half = CHUNK as i32 / 2;
        let far = |cells: &Cells, (site, chunk): (usize, [i32; 3])| {
            cells.sites[site]
                .corner(chunk.map(|n| n + half))
                .distance(eye)
        };
        let mut nearest: Vec<(usize, [i32; 3])> = cells.stale.iter().copied().collect();
        nearest.sort_by(|&a, &b| far(&cells, a).total_cmp(&far(&cells, b)));
        // The update it landed in meshes nothing of it.
        cells.update(eye);
        assert!(cells.drain_changes().is_empty());
        assert_eq!(cells.stale.len(), owed);
        // The next meshes the nearest of what is owed, and no more.
        cells.update(eye);
        assert!(cells.drain_changes().len() <= MESHES_PER_UPDATE);
        let left: BTreeSet<_> = nearest[MESHES_PER_UPDATE..].iter().copied().collect();
        assert_eq!(cells.stale, left);
        let mut updates = 1;
        while !cells.settled() {
            cells.update(eye);
            updates += 1;
        }
        assert_eq!(updates, owed.div_ceil(MESHES_PER_UPDATE));
        assert!(!cells.drawn.is_empty());
    }

    #[test]
    fn a_gesture_stands_over_two_neighbours_and_is_taken_back_as_one() {
        let (generator, mut cells, origin) = opened();
        let (_, point) = world();
        cells
            .open(&generator, point.sector.into(), along(point, 1))
            .unwrap();
        // From the floor out over the plot beside it, where there is none.
        let row = create(at(origin, [60, 20, 0]), at(origin, [67, 20, 0]), 4);
        assert!(cells.apply(&generator, point.sector, &[row]));
        for x in 60..=67 {
            let cell = cells.cell(point.sector, at(origin, [x, 20, 0]));
            assert_eq!(cell.paint(), Some(4), "{x}");
        }
        cells.take_back(&generator);
        for x in 60..=67 {
            assert!(cells.cell(point.sector, at(origin, [x, 20, 0])).is_air());
        }
    }

    #[test]
    fn a_gesture_stops_where_no_volume_stands() {
        let (generator, mut cells, origin) = opened();
        let sector = cells.sites[0].seat.sector();
        let row = create(at(origin, [60, 20, 0]), at(origin, [67, 20, 0]), 0);
        assert!(cells.apply(&generator, sector, &[row]));
        let held: Vec<[i32; 3]> = (60..64).map(|x| [x, 20, 0]).collect();
        assert_eq!(laid(&cells, origin), held);
        assert!(cells.holds(sector, at(origin, [63, 20, 0])));
        assert!(!cells.holds(sector, at(origin, [64, 20, 0])));
        // A sector nothing is built on holds nothing, and takes nothing.
        let other = Sector::new(1).unwrap();
        assert!(!cells.apply(&generator, other, &[row]));
        assert!(cells.cell(other, at(origin, [60, 20, 0])).is_air());
    }

    #[test]
    fn a_body_stands_astride_two_volumes() {
        let (generator, mut cells, origin) = opened();
        let (_, point) = world();
        cells
            .open(&generator, point.sector.into(), along(point, 1))
            .unwrap();
        // One cube at the edge of the second volume, level with the floor of
        // the first, and a body half on it, its middle over the first.
        let cube = at(origin, [64, 20, -1]);
        cells.apply(&generator, point.sector, &[create(cube, cube, 0)]);
        let floor_m = f64::from(origin[2]) * BLOCK_M;
        let beside = |du: f64| {
            let (u, v) = (f64::from(cube[0]) + du, f64::from(cube[1]) + 0.5);
            cells.footing(
                point.sector.into(),
                SurfacePoint::new(point.sector, u, v),
                floor_m,
            )
        };
        assert_eq!(beside(-0.3).unwrap().floor_m, Some(floor_m));
        // Its middle over the cube, nothing else of it on anything built.
        assert_eq!(beside(0.7).unwrap().floor_m, Some(floor_m));
        // Past the cube the second volume holds nothing up.
        assert_eq!(beside(2.0).unwrap().floor_m, None);
        // And off every volume there is no footing to speak of.
        let off = SurfacePoint::new(point.sector, f64::from(origin[0]) - 5.0, point.v);
        assert_eq!(cells.footing(off.sector.into(), off, floor_m), None);
    }

    #[test]
    fn a_body_stands_on_what_is_laid() {
        let (generator, mut cells, origin) = opened();
        let sector = cells.sites[0].seat.sector();
        let row = create(at(origin, [10, 20, 0]), at(origin, [14, 20, 0]), 4);
        cells.apply(&generator, sector, &[row]);
        // One gesture is drawn in the update it landed in.
        cells.update(top(&cells, at(origin, [12, 20, 8])));
        assert!(!cells.drawn.is_empty());
        let on = SurfacePoint::new(
            sector,
            f64::from(origin[0]) + 12.5,
            f64::from(origin[1]) + 20.5,
        );
        let footing = cells
            .footing(on.sector.into(), on, f64::from(origin[2]) * BLOCK_M)
            .unwrap();
        assert_eq!(footing.floor_m, Some(f64::from(origin[2] + 1) * BLOCK_M));
    }

    #[test]
    fn the_ground_of_a_volume_is_aimed_at_and_hides_the_cell_under_it() {
        let (generator, mut cells, origin) = opened();
        let (_, point) = world();
        // A cube sunk two metres into the ground, under an eye over it: the
        // eye meets the ground of the volume, where a cell would stand in
        // it, and never the cube.
        let (x, y) = (origin[0] + 70, origin[1] + 20);
        let column = SurfacePoint::new(point.sector, f64::from(x), f64::from(y));
        let under = ground(&generator, column.sector.into(), column).floor() as i32;
        let sunk = [x, y, under - 4];
        cells
            .open(&generator, point.sector.into(), along(point, 1))
            .unwrap();
        cells.apply(&generator, point.sector, &[create(sunk, sunk, 0)]);
        let over = top(&cells, [x, y, under + 20]);
        let down = (top(&cells, sunk) - over).normalize();
        let aim = cells
            .sight(&generator, over, down, None)
            .aim()
            .expect("the ground");
        assert_eq!(aim.met, Met::Ground);
        assert_eq!(aim.hit.before(), [x, y, under]);
        assert!(cells.cell(aim.seat, aim.hit.before()).is_air());
        // Where no volume stands the ground is nothing to aim at, and hides
        // what is behind it all the same.
        let seat = Seat::Sector(point.sector);
        cells.close(seat, along(point, 1));
        assert_eq!(cells.sight(&generator, over, down, None).aim(), None);
        // The floor beside it is in plain sight.
        let seen = at(origin, [30, 20, -1]);
        let floor = top(&cells, seen);
        let over = floor + floor.normalize() * 5.0;
        let sight = cells.sight(&generator, over, -floor.normalize(), None);
        let aim = sight.aim().expect("the floor");
        assert_eq!((aim.seat, aim.hit.cell), (point.sector.into(), seen));
        assert_eq!(aim.met, Met::Cell);
        // The address says where a corner of a cell is, as its sides are drawn.
        let corner = cells.corner(&generator, aim.seat, seen);
        assert_eq!(corner, cells.sites[0].corner(seen));
    }

    #[test]
    fn the_ghost_shows_a_gesture_and_goes() {
        let (_, mut cells, origin) = opened();
        let sector = cells.sites[0].seat.sector();
        let cube = at(origin, [3, 3, 0]);
        cells.preview(Some((sector.into(), create(cube, cube, 0))));
        assert_eq!(cells.ghost().map(|draw| draw.id), Some(GHOST));
        cells.preview(None);
        assert_eq!(cells.ghost(), None);
        // Over a sector nothing is built on there is nothing to show.
        let other = Sector::new(1).unwrap();
        cells.preview(Some((other.into(), create(cube, cube, 0))));
        assert_eq!(cells.ghost(), None);
    }

    #[test]
    fn the_ghost_is_meshed_again_after_a_change_lands() {
        let (generator, mut cells, origin) = opened();
        let sector = cells.sites[0].seat.sector();
        let cube = at(origin, [3, 3, 0]);
        cells.apply(&generator, sector, &[create(cube, cube, 0)]);
        let delete = Gesture::Delete {
            span: Span::cell(cube),
        };
        cells.preview(Some((sector.into(), delete)));
        cells.drain_changes();
        cells.apply(&generator, sector, &[delete]);
        cells.preview(Some((sector.into(), delete)));
        // Whatever the last frame shows over the hole, it is not the cube.
        let ghost = cells
            .drain_changes()
            .into_iter()
            .rev()
            .find_map(|change| match change {
                VolumeChange::Add(GHOST, mesh) => Some(Some(mesh)),
                VolumeChange::Remove(GHOST) => Some(None),
                _ => None,
            });
        assert!(ghost.is_some_and(|mesh| mesh.is_none_or(|mesh| mesh.indices.is_empty())));
    }

    #[test]
    fn a_change_is_taken_back_and_put_back() {
        let (generator, mut cells, origin) = opened();
        let sector = cells.sites[0].seat.sector();
        let row = create(at(origin, [5, 5, 0]), at(origin, [7, 5, 0]), 0);
        cells.apply(&generator, sector, &[row]);
        let middle = at(origin, [6, 5, 0]);
        assert_eq!(cells.history(), (true, false));
        cells.take_back(&generator);
        assert!(cells.cell(sector, middle).is_air());
        assert_eq!(cells.history(), (false, true));
        cells.put_back(&generator);
        assert!(!cells.cell(sector, middle).is_air());
        // A new change forgets what was taken back.
        cells.take_back(&generator);
        let cube = at(origin, [9, 9, 0]);
        cells.apply(&generator, sector, &[create(cube, cube, 0)]);
        assert_eq!(cells.history(), (true, false));
        // What changes nothing is no change to take back.
        cells.take_back(&generator);
        assert!(!cells.apply(&generator, sector, &[]));
        let air = Gesture::Delete {
            span: Span::cell(cube),
        };
        assert!(!cells.apply(&generator, sector, &[air]));
        assert_eq!(cells.history(), (false, true));
    }

    #[test]
    fn a_chunk_has_a_mesh_of_its_own_in_every_volume_and_seat() {
        let sphere = QuadSphere::new(16).unwrap();
        let seated = |seat: fn(Sector) -> Seat, sector: u8| Seated {
            seat: seat(Sector::new(sector).unwrap()),
            sphere,
            scale: 1,
            volumes: Volumes::new(PLOT_BITS),
            balls: BTreeMap::new(),
        };
        let chunks = [[64, 128, 32], [80, 128, 32], [64, 144, 32], [64, 128, -32]];
        let mut ids = BTreeSet::new();
        for seat in [Seat::Sector, Seat::Moon] {
            for sector in [0, 5] {
                for chunk in chunks {
                    ids.insert(seated(seat, sector).mesh_id(chunk));
                }
            }
        }
        assert_eq!(ids.len(), 16);
        assert!(!ids.contains(&GHOST));
        // The glass of a chunk is drawn beside its cubes, under an id of
        // its own.
        assert!(ids.iter().all(|id| id.0 & GLASS == 0));
    }

    #[test]
    fn sides_share_their_corners_across_cells() {
        let (_, cells, origin) = opened();
        let site = &cells.sites[0];
        let a = site.corner(at(origin, [3, 4, 2]));
        let quads = [
            Quad {
                cell: at(origin, [2, 3, 1]),
                face: voxel::Face::UP,
                paint: 0,
                open: [3; 4],
            },
            Quad {
                cell: at(origin, [3, 4, 1]),
                face: voxel::Face::UP,
                paint: 0,
                open: [3; 4],
            },
        ];
        let mesh = mesh(site.seat.sector(), site.sphere, 1, &quads, paint_look, 0.0);
        let shared: Vec<DVec3> = mesh
            .vertices
            .iter()
            .map(|v| mesh.origin + Vec3::from(v.position).as_dvec3())
            .filter(|p| p.distance(a) < 1e-3)
            .collect();
        assert_eq!(shared.len(), 2);
        // Up is out of the planet.
        let normal = Vec3::from(mesh.vertices[0].normal).as_dvec3();
        assert!(normal.dot(a.normalize()) > 0.99);
    }
}
