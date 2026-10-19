//! The parts of a world on the client: the placed things that are no cell,
//! seated on the planet and on the moon, and the models that draw them
//! (DECISIONS 111).
//!
//! A part knows no sphere (`part`). Here it is seated: its pose is said in
//! the flat frame of a point of its seat, and `seat::seated` puts it in the
//! world, rigid, so a part is moved in the world and its pose read back from
//! where it came to (`seat::reseat`). A part stands anywhere on its seat,
//! over a volume, between two, on nature: its place is an address with
//! fractional blocks, never a volume.
//!
//! The parts are a system of the core (DECISIONS 108): anyone reads what
//! they hold, and an op alone changes it. A hand that places, moves and
//! turns them is a plugin's, and it speaks to them through its host
//! (`crate::plugin`): pick, place, set, remove.
//!
//! A part is drawn as a model: a mesh in its own space, a cube of one voxel
//! in one paint, placed by one matrix a frame. A cube's mesh is made once a
//! paint and shared by every cube in it. A model of a file (`Shape::Vox`) is
//! asked for over the asset seam when a part first names it, meshed once it
//! arrives, and shared by every part that names it; until it arrives the
//! part stands undrawn, and its box holds a body up. A picture is drawn
//! apart, by `pictures`.
//!
//! In a world, the world keeps the parts (DECISIONS 111): a part placed here
//! is placed at once and asked of the world, which names it; what the world
//! calls a part is the link's business, and a hand knows a part by the name
//! given here. A set or a remove of a part the world has not named yet
//! waits for the name. What others change arrives as events, and the parts
//! near the body are asked for every second. With no world, a headless
//! picture or a bench, the parts are held here alone.

use std::collections::{BTreeMap, HashMap};

use glam::{DAffine3, DMat3, DVec3};
use part::{Gap, Part, PartId, Portal, Pose, Shape};
use protocol::parts as wire;
use protocol::{Body, Message};
use scene::{
    Lamp, ModelChange, ModelDraw, ModelId, ModelMesh, ModelPass, PaneDraw, PictureDraw,
    VolumeVertex,
};
use seat::Seat;
use topology::{BLOCK_M, SurfacePoint};
use voxel::{Finish, Paint};
use worldgen::Generator;

use crate::assets::Requests;
use crate::cells::{
    Ask, LAMP_FAR_M, LAMP_GAIN, LAMP_GROWS_M, LAMP_REACH_M, LAMP_SIDES, paint_look, paint_rgb,
};
use crate::collision::{BODY_HALF, Footing, STEP_M};
use crate::pictures::{Pictures, Placed, Screen};
use crate::speakers::Speakers;

/// Seconds between two looks at what the world holds near the body.
const LOOK_S: f64 = 1.0;
/// How far a body goes from a part before its client lets it go, metres:
/// past where the world shows it (`world::parts::HOLD_M`) by the same
/// margin the cells keep for a volume seen afar (`seat::AFAR`). How far
/// one is drawn within that is the eye's own, `Effects::parts_reach_m`.
const DROP_M: f64 = 8448.0;
/// What a part whose file is still on its way is drawn as: its box in white
/// glass with a white edge, and over it a white light turning once a
/// second, so the place it will fill is seen and seen to be waiting.
const LOADING_PAINT: u8 = Paint {
    color: 0,
    finish: Finish::Glass,
    edge: voxel::Edge::White,
}
.byte();
const WAITING_PAINT: u8 = Paint {
    color: 0,
    finish: Finish::Light,
    edge: voxel::Edge::None,
}
.byte();
/// The mark turning over a loading part: how long and how thick, metres,
/// how high over the box, and turns a second.
const WAITING_BAR_M: f64 = 0.5;
const WAITING_THICK_M: f64 = 0.06;
const WAITING_OVER_M: f64 = 0.25;
const WAITING_TURNS_HZ: f64 = 1.0;
/// Blocks along the side of a bucket of a seat's grid: a part is in every
/// bucket its box and a body's width reach, so a body's column asks the
/// parts of its own bucket and no other.
const BUCKET: f64 = 16.0;
/// Seconds between two sets of one part sent to the world: a drag sets a
/// part every frame, and the world hears the latest ten times a second.
const SET_S: f64 = 0.1;

/// The size of a cube as a hand places it, in blocks: a metre.
pub const CUBE: [f64; 3] = [2.0, 2.0, 2.0];

/// The size of a model's voxel as a hand places it, in blocks: an eighth of
/// a cell, 6.25 cm, so a model of 32 voxels stands 2 m tall
/// (docs/sketches/entities.md § The pose).
pub const VOXEL: f64 = 1.0 / 8.0;

/// Why the parts refuse what was asked of them. A plugin says it at the
/// seam under its own name, and a front end in its own words.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PartRefusal {
    /// Placing is a builder's and an admin's, and the world named this
    /// session a lower level, or no world has spoken (DECISIONS 104).
    Level,
    /// The pose is no pose: not finite, not a turn, or flat.
    Pose,
    /// No part of that name stands.
    Gone,
}

/// The models of the core's own shapes have ids under this; a model of a
/// file is named by the hash of its content, over it.
const SHAPES: u64 = 1 << 24;
/// The shape numbers of the rings: one a width of rim, over the others'.
const RINGS: u64 = 1 << 8;
/// The bit that marks the model of a file.
const FILE: u64 = 1 << 63;

/// A set or a remove asked before the world named the part.
enum Deferred {
    /// Boxed: a part is a good deal larger than nothing.
    Set(Box<(Seat, Part)>),
    Remove,
}

/// A change made here with no world, as what takes it back and puts it
/// back (DECISIONS 115). In a world the world keeps these.
enum Change {
    Placed(PartId),
    /// A part set: as it was before.
    Set(PartId, Seat, Part),
    /// A part removed: as it was.
    Removed(PartId, Seat, Part),
}

/// The parts as a world keeps them, from where this client stands.
#[derive(Default)]
struct Link {
    /// This client's own session: a part placed under that number is one
    /// it placed.
    me: u32,
    /// The last number given to an op.
    asked: u32,
    asks: Vec<Ask>,
    /// Places asked that the world has not named, the first first: the
    /// first `placed` of this session's own is the first of them.
    placing: Vec<(u32, PartId)>,
    /// Sets and removes asked, by number, each of which part by the
    /// world's name: refused, what the world holds of it is asked for
    /// again.
    waiting: HashMap<u32, u64>,
    /// What the world calls each part held here, and back.
    world_of: HashMap<PartId, u64>,
    local_of: HashMap<u64, PartId>,
    /// The version the world last said of each part, by its world name.
    versions: HashMap<u64, u64>,
    /// What was asked of a part before the world named it, in order.
    deferred: Vec<(PartId, Deferred)>,
    /// The latest set of each part since the last cadence, waiting to go.
    dirty: BTreeMap<PartId, (Seat, Part)>,
    since_look_s: f64,
    since_set_s: f64,
}

impl Link {
    /// Queues an op for the world, under the next number.
    fn ask<M: Message>(&mut self, kind: &'static str, op: &M) -> u32 {
        self.asked = self.asked.wrapping_add(1).max(1);
        let id = self.asked;
        self.asks.push(Ask {
            kind,
            payload: op.encode_to_vec(),
            id,
        });
        id
    }
}

/// How long before the chart is asked again for a landmark not held here,
/// seconds.
const CHART_AGAIN_S: f64 = 2.0;

/// A landmark as the chart says it: under the name the world gives it, or
/// the name here with no world, where it stands and how far from the body.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Landmarked {
    pub id: u64,
    pub seat: Seat,
    pub part: Part,
    /// From the body, metres; infinite from another body.
    pub away_m: f64,
}

/// Every part held, by its name, and what draws them.
#[derive(Default)]
pub struct Parts {
    held: BTreeMap<PartId, (Seat, Part)>,
    /// The chart's answer since last taken: the landmarks of the world.
    charted: Option<Vec<(u64, Seat, Part)>>,
    /// The landmarks of the world as last charted, by name: where a portal
    /// leads when its star is not held here.
    known: BTreeMap<u64, (Seat, Part)>,
    /// When the chart was last asked for a landmark not held here, seconds
    /// of the clock.
    asked_s: Option<f64>,
    /// The next name given here: a hand knows a part by it, in a world or
    /// alone.
    next: u64,
    /// The world that keeps the parts, while one does.
    link: Option<Link>,
    /// With no world: the changes made here to take back and put back.
    done: Vec<Change>,
    undone: Vec<Change>,
    /// Every part by the buckets of its seat's grid its box reaches, and
    /// which buckets each is in, to take it out again.
    buckets: HashMap<(Seat, [i32; 2]), Vec<PartId>>,
    bucketed: HashMap<PartId, Vec<(Seat, [i32; 2])>>,
    /// The parts whose box reaches past the edge of their sector: a body on
    /// the next sector may stand on them, so every column of their body
    /// asks them.
    edges: Vec<PartId>,
    /// The models announced, by id.
    meshed: Vec<ModelId>,
    changes: Vec<ModelChange>,
    /// The grid of every model of a file that arrived, by the file's hash.
    grids: HashMap<[u8; 32], vox::Grid>,
    /// The files asked for, so none is asked for twice.
    asked: Vec<[u8; 32]>,
    /// The files to ask for, since last asked.
    wanted: Vec<[u8; 32]>,
    /// Where the moon is now, from the planet's centre.
    moon: DVec3,
    /// The marks a hand shows over the world this frame: cubes in a paint,
    /// each where its transform puts it, drawn over everything.
    marks: Vec<ModelDraw>,
}

/// A mark a hand shows over the world: a unit cube in a paint, placed by a
/// world transform from the planet's centre, drawn over everything and
/// cast by nothing. The handles of a gizmo are marks.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Mark {
    pub paint: u8,
    pub transform: DAffine3,
}

impl Parts {
    /// The moon is somewhere else: what is seated on it goes with it.
    pub(crate) fn orbit(&mut self, moon: DVec3) {
        self.moon = moon;
    }

    /// Holds a part under a name, in its buckets: the one it replaces, if any.
    fn hold(
        &mut self,
        generator: &Generator,
        id: PartId,
        seat: Seat,
        part: Part,
    ) -> Option<(Seat, Part)> {
        let before = self.let_go(id);
        self.held.insert(id, (seat, part));
        let side = f64::from(seat.sphere(generator).blocks().side());
        let reach = part.reach() + BODY_HALF;
        let [u, v, _] = part.pose.at;
        if u - reach < 0.0 || v - reach < 0.0 || u + reach > side || v + reach > side {
            self.edges.push(id);
        }
        let bucket = |n: f64| (n / BUCKET).floor() as i32;
        let (lo, hi) = (
            [bucket(u - reach), bucket(v - reach)],
            [bucket(u + reach), bucket(v + reach)],
        );
        let mut cells = Vec::new();
        for y in lo[1]..=hi[1] {
            for x in lo[0]..=hi[0] {
                self.buckets.entry((seat, [x, y])).or_default().push(id);
                cells.push((seat, [x, y]));
            }
        }
        self.bucketed.insert(id, cells);
        before
    }

    /// Lets go of a part by its name, out of its buckets: what it was.
    fn let_go(&mut self, id: PartId) -> Option<(Seat, Part)> {
        let was = self.held.remove(&id)?;
        for cell in self.bucketed.remove(&id).unwrap_or_default() {
            if let Some(bucket) = self.buckets.get_mut(&cell) {
                bucket.retain(|held| *held != id);
                if bucket.is_empty() {
                    self.buckets.remove(&cell);
                }
            }
        }
        self.edges.retain(|held| *held != id);
        Some(was)
    }

    /// A world keeps the parts from here on: what was held is forgotten,
    /// and what the world holds near the body is asked for at once.
    pub(crate) fn link(&mut self, me: u32) {
        self.held.clear();
        self.done.clear();
        self.undone.clear();
        self.link = Some(Link {
            me,
            since_look_s: LOOK_S,
            ..Link::default()
        });
    }

    /// The world is gone, and what it kept with it.
    pub(crate) fn unlink(&mut self) {
        self.held.clear();
        self.done.clear();
        self.undone.clear();
        self.link = None;
    }

    /// The number the last op was asked of the world under. `None` with no
    /// world, or when nothing went out.
    pub(crate) fn last_ask(&self) -> Option<u32> {
        self.link
            .as_ref()
            .and_then(|link| link.asks.last().map(|ask| ask.id))
    }

    /// Takes back the last change made here, or puts back the last taken
    /// back. In a world, the world is asked to, and says what the parts are
    /// then; alone, the stacks here say.
    pub(crate) fn restore(&mut self, generator: &Generator, back: bool) {
        if let Some(link) = &mut self.link {
            let asked = match back {
                true => link.ask(wire::TAKE_BACK, &wire::TakeBack {}),
                false => link.ask(wire::PUT_BACK, &wire::PutBack {}),
            };
            link.waiting.insert(asked, 0);
            return;
        }
        let change = match back {
            true => self.done.pop(),
            false => self.undone.pop(),
        };
        let Some(change) = change else {
            return;
        };
        let inverse = match change {
            Change::Placed(id) => self
                .let_go(id)
                .map(|(seat, part)| Change::Removed(id, seat, part)),
            Change::Removed(id, seat, part) => {
                self.hold(generator, id, seat, part);
                self.mesh(&part);
                Some(Change::Placed(id))
            }
            Change::Set(id, seat, part) => {
                let before = self.hold(generator, id, seat, part);
                self.mesh(&part);
                before.map(|(seat, part)| Change::Set(id, seat, part))
            }
        };
        if let Some(inverse) = inverse {
            match back {
                true => self.undone.push(inverse),
                false => self.done.push(inverse),
            }
        }
    }

    /// Asks the world for its landmarks, wherever they stand; alone, the
    /// ones held here are the answer at once. What comes is taken with
    /// [`Parts::take_charted`].
    pub(crate) fn chart(&mut self) {
        if let Some(link) = &mut self.link {
            link.ask(wire::CHART, &wire::Chart {});
            return;
        }
        let landmarks: Vec<(u64, Seat, Part)> = self
            .all()
            .filter(|(_, _, part)| matches!(part.shape, Shape::Landmark(_)))
            .map(|(id, seat, part)| (id.0, seat, *part))
            .collect();
        self.know(&landmarks);
        self.charted = Some(landmarks);
    }

    /// Asks for the chart for a landmark not held here, at most once every
    /// [`CHART_AGAIN_S`]: what a portal near the body leads to.
    pub(crate) fn chart_for(&mut self, clock_s: f64) {
        if self
            .asked_s
            .is_none_or(|asked_s| clock_s - asked_s >= CHART_AGAIN_S)
        {
            self.asked_s = Some(clock_s);
            self.chart();
        }
    }

    fn know(&mut self, landmarks: &[(u64, Seat, Part)]) {
        self.known = landmarks
            .iter()
            .map(|&(id, seat, part)| (id, (seat, part)))
            .collect();
    }

    /// A landmark by its name: held here, else as the chart last said it.
    pub(crate) fn landmark(&self, id: PartId) -> Option<(Seat, Part)> {
        match self.get(id) {
            Some((seat, part)) if matches!(part.shape, Shape::Landmark(_)) => Some((seat, *part)),
            _ => self.known.get(&id.0).copied(),
        }
    }

    /// Where a landmark stands in the world: its star's frame.
    pub(crate) fn landmark_transform(&self, generator: &Generator, id: PartId) -> Option<DAffine3> {
        let (seat, part) = self.landmark(id)?;
        Some(self.place_of(generator, seat, &part.pose))
    }

    /// The panes of the portals in reach: the disc inside each ring, placed
    /// as a picture's quad is, facing the ring's front. `seen` says how
    /// much the world is seen through each, by name; none is a lens.
    pub fn panes(
        &self,
        generator: &Generator,
        body: (Body, DVec3),
        reach_m: f64,
        seen: &BTreeMap<PartId, f32>,
    ) -> Vec<PaneDraw> {
        // The quad's `x` along the box's, its `y` up the box, its front out
        // of the box's front (`-y`), the disc as wide as the rim leaves.
        let quad_in_box = |portal: Portal| {
            let across = f64::from(1.0 - 2.0 * portal.rim_of_width());
            DAffine3::from_cols(
                DVec3::new(across, 0.0, 0.0),
                DVec3::new(0.0, 0.0, across),
                DVec3::new(0.0, -1.0, 0.0),
                DVec3::new(0.0, 0.0, 0.5),
            )
        };
        self.all()
            .filter_map(|(id, seat, part)| match part.shape {
                Shape::Portal(portal) => Some((id, seat, part, portal)),
                _ => None,
            })
            .filter(|(_, seat, part, _)| away_m(generator, *seat, &part.pose, body) <= reach_m)
            .map(|(id, seat, part, portal)| PaneDraw {
                transform: self.place_of(generator, seat, &part.pose) * quad_in_box(portal),
                seen: seen.get(&id).copied().unwrap_or(0.0),
            })
            .collect()
    }

    /// The chart's answer, once: the landmarks, the nearest the body first,
    /// those on another body last. `body` is the body the client's own
    /// stands on and where, from its centre.
    pub(crate) fn take_charted(
        &mut self,
        generator: &Generator,
        body: (Body, DVec3),
    ) -> Option<Vec<Landmarked>> {
        let mut landmarks: Vec<Landmarked> = self
            .charted
            .take()?
            .into_iter()
            .map(|(id, seat, part)| Landmarked {
                id,
                seat,
                part,
                away_m: away_m(generator, seat, &part.pose, body),
            })
            .collect();
        landmarks.sort_by(|a, b| a.away_m.total_cmp(&b.away_m).then(a.id.cmp(&b.id)));
        Some(landmarks)
    }

    /// What to ask the world since the last call, in order.
    pub(crate) fn drain_asks(&mut self) -> Vec<Ask> {
        self.link
            .as_mut()
            .map(|link| core::mem::take(&mut link.asks))
            .unwrap_or_default()
    }

    /// Sends the world the sets that waited for the cadence, and once a
    /// second lets go of the parts far behind the body and asks the world
    /// for the parts near it this client lacks or holds stale. `body` is
    /// the body the client's own stands on and where, from its centre.
    pub(crate) fn look(&mut self, generator: &Generator, body: (Body, DVec3), dt: f64) {
        let Some(link) = &mut self.link else {
            return;
        };
        link.since_set_s += dt;
        if link.since_set_s >= SET_S {
            link.since_set_s = 0.0;
            for (local, (seat, part)) in core::mem::take(&mut link.dirty) {
                self.send(local, Deferred::Set(Box::new((seat, part))));
            }
        }
        let Some(link) = &mut self.link else {
            return;
        };
        link.since_look_s += dt;
        if link.since_look_s < LOOK_S {
            return;
        }
        link.since_look_s = 0.0;
        // A part far behind the body is let go, and asked for again when
        // the body comes back.
        let far: Vec<PartId> = self
            .held
            .iter()
            .filter(|(_, (seat, part))| away_m(generator, *seat, &part.pose, body) > DROP_M)
            .map(|(&id, _)| id)
            .collect();
        for local in far {
            self.forget(local);
        }
        let Some(link) = &mut self.link else {
            return;
        };
        let held = link
            .versions
            .iter()
            .map(|(&id, &version)| wire::Held { id, version })
            .collect();
        link.ask(wire::LOOK, &wire::Look { held });
    }

    /// An event of the world's parts.
    pub(crate) fn receive(&mut self, generator: &Generator, kind: &str, payload: &[u8]) {
        let Some(me) = self.link.as_ref().map(|link| link.me) else {
            return;
        };
        match kind {
            wire::PLACED => {
                let Ok(placed) = wire::Placed::decode(payload) else {
                    return;
                };
                let Some(record) = &placed.record else {
                    return;
                };
                if placed.session == me {
                    // The first place of this session's own that the world
                    // has not named: this is its name.
                    let link = self.link.as_mut().expect("linked");
                    if !link.placing.is_empty() {
                        let (_, local) = link.placing.remove(0);
                        self.name(local, record);
                        return;
                    }
                }
                self.arrived(generator, record);
            }
            wire::CHANGED => {
                let Ok(changed) = wire::Changed::decode(payload) else {
                    return;
                };
                let Some(record) = &changed.record else {
                    return;
                };
                // A change of this session's own is held already while a
                // set of it is on its way; otherwise it is the world moving
                // the part, a change taken back, and it lands.
                let link = self.link.as_mut().expect("linked");
                let on_its_way = link.local_of.get(&record.id).is_some_and(|local| {
                    link.dirty.contains_key(local)
                        || link.waiting.values().any(|id| id == &record.id)
                });
                match changed.session == me && on_its_way {
                    true => {
                        link.versions.insert(record.id, record.version);
                    }
                    false => self.arrived(generator, record),
                }
            }
            wire::REMOVED => {
                if let Ok(removed) = wire::Removed::decode(payload) {
                    self.gone(removed.id);
                }
            }
            wire::SEEN => {
                let Ok(seen) = wire::Seen::decode(payload) else {
                    return;
                };
                for record in &seen.records {
                    self.arrived(generator, record);
                }
                for id in seen.gone {
                    self.gone(id);
                }
            }
            wire::CHARTED => {
                let Ok(charted) = wire::Charted::decode(payload) else {
                    return;
                };
                let landmarks: Vec<(u64, Seat, Part)> = charted
                    .records
                    .iter()
                    .filter_map(|record| {
                        let (seat, part) = seat::part_from_wire(record.part.as_ref()?)?;
                        Some((record.id, seat, part))
                    })
                    .collect();
                self.know(&landmarks);
                self.charted = Some(landmarks);
            }
            _ => {}
        }
    }

    /// The world named a part placed here: what was asked of it meanwhile
    /// goes out under that name.
    fn name(&mut self, local: PartId, record: &wire::Record) {
        let link = self.link.as_mut().expect("linked");
        link.world_of.insert(local, record.id);
        link.local_of.insert(record.id, local);
        link.versions.insert(record.id, record.version);
        let deferred: Vec<Deferred> = {
            let (mine, rest): (Vec<_>, Vec<_>) = core::mem::take(&mut link.deferred)
                .into_iter()
                .partition(|(id, _)| *id == local);
            link.deferred = rest;
            mine.into_iter().map(|(_, deferred)| deferred).collect()
        };
        for deferred in deferred {
            self.send(local, deferred);
        }
    }

    /// Asks the world for a set or a remove of a part it has named, or
    /// keeps it for when it has.
    fn send(&mut self, local: PartId, deferred: Deferred) {
        let Some(link) = &mut self.link else {
            return;
        };
        let Some(&id) = link.world_of.get(&local) else {
            link.deferred.push((local, deferred));
            return;
        };
        let asked = match deferred {
            Deferred::Set(set) => link.ask(
                wire::SET,
                &wire::Set {
                    id,
                    part: Some(seat::part_wire(set.0, &set.1)),
                },
            ),
            Deferred::Remove => link.ask(wire::REMOVE, &wire::Remove { id }),
        };
        link.waiting.insert(asked, id);
    }

    /// A part as the world holds it: held here under the name it has here,
    /// or a new one.
    fn arrived(&mut self, generator: &Generator, record: &wire::Record) {
        let Some((seat, part)) = record.part.as_ref().and_then(seat::part_from_wire) else {
            return;
        };
        let link = self.link.as_mut().expect("linked");
        let stale = link.versions.get(&record.id) != Some(&record.version);
        link.versions.insert(record.id, record.version);
        let local = match link.local_of.get(&record.id) {
            Some(&local) => {
                if !stale {
                    return;
                }
                local
            }
            None => {
                self.next += 1;
                let local = PartId(self.next);
                link.local_of.insert(record.id, local);
                link.world_of.insert(local, record.id);
                local
            }
        };
        self.hold(generator, local, seat, part);
        self.mesh(&part);
    }

    /// Lets go of a part held here, by its name here: what the world calls
    /// it is forgotten too, so a look brings it again when it is near.
    fn forget(&mut self, local: PartId) {
        self.let_go(local);
        if let Some(link) = &mut self.link {
            link.dirty.remove(&local);
            link.deferred.retain(|(part, _)| *part != local);
            if let Some(id) = link.world_of.remove(&local) {
                link.local_of.remove(&id);
                link.versions.remove(&id);
            }
        }
    }

    /// A part the world no longer holds.
    fn gone(&mut self, id: u64) {
        let Some(link) = &mut self.link else {
            return;
        };
        link.versions.remove(&id);
        if let Some(local) = link.local_of.remove(&id) {
            link.world_of.remove(&local);
            link.dirty.remove(&local);
            link.deferred.retain(|(part, _)| *part != local);
            self.let_go(local);
        }
    }

    /// The world answered an op of the parts: landed, with no code, or
    /// refused with one. A place refused takes the part back; a set or a
    /// remove refused asks for what is held again. False for a number that
    /// is none of the parts'.
    pub(crate) fn answered(&mut self, id: u32, code: &str) -> bool {
        let Some(link) = &mut self.link else {
            return false;
        };
        if let Some(at) = link.placing.iter().position(|(asked, _)| *asked == id) {
            let (_, local) = link.placing.remove(at);
            if !code.is_empty() {
                link.deferred.retain(|(part, _)| *part != local);
                self.let_go(local);
            }
            return true;
        }
        if let Some(world) = link.waiting.remove(&id) {
            // Refused: what is held of it is stale by the world's word, so
            // the next look brings the part as the world has it.
            if !code.is_empty() {
                link.versions.remove(&world);
                link.since_look_s = LOOK_S;
            }
            return true;
        }
        false
    }

    fn centre(&self, body: Body) -> DVec3 {
        match body {
            Body::Moon => self.moon,
            Body::Planet => DVec3::ZERO,
        }
    }

    /// The seat and pose a part standing in the world has: the world
    /// transform from the planet's centre read back onto a body
    /// (`seat::reseat`). `None` for a transform that is no pose.
    pub fn reseat(
        &self,
        generator: &Generator,
        body: Body,
        world: &DAffine3,
    ) -> Option<(Seat, Pose)> {
        let on_body = DAffine3::from_translation(-self.centre(body)) * *world;
        let (sector, pose) = seat::reseat(seat::sphere_of(generator, body), &on_body)?;
        Some((Seat::on(body, sector), pose))
    }

    /// Shows marks over the world, in place of those shown before, and none
    /// with none to show.
    pub(crate) fn mark(&mut self, marks: &[Mark]) {
        self.marks = marks
            .iter()
            .map(|mark| ModelDraw {
                mesh: self
                    .ensure(Shape::Cube, mark.paint)
                    .expect("a cube is a model"),
                transform: mark.transform,
                pass: ModelPass::Over,
            })
            .collect();
    }

    /// A part by its name, and the seat it stands on.
    pub fn get(&self, id: PartId) -> Option<(Seat, &Part)> {
        self.held.get(&id).map(|(seat, part)| (*seat, part))
    }

    /// Every part held, with its name and seat.
    pub fn all(&self) -> impl Iterator<Item = (PartId, Seat, &Part)> {
        self.held
            .iter()
            .map(|(id, (seat, part))| (*id, *seat, part))
    }

    /// Whether any part stands on a body.
    pub fn stands_on(&self, body: Body) -> bool {
        self.held.values().any(|(seat, _)| seat.body() == body)
    }

    /// Places a part on a seat and names it. In a world, the world is asked
    /// to place it too, and names it in its own way.
    pub(crate) fn place(&mut self, generator: &Generator, seat: Seat, part: Part) -> PartId {
        self.next += 1;
        let id = PartId(self.next);
        self.hold(generator, id, seat, part);
        self.mesh(&part);
        match &mut self.link {
            Some(link) => {
                let place = wire::Place {
                    part: Some(seat::part_wire(seat, &part)),
                };
                let asked = link.ask(wire::PLACE, &place);
                link.placing.push((asked, id));
            }
            None => {
                self.undone.clear();
                self.done.push(Change::Placed(id));
            }
        }
        id
    }

    /// Replaces a part, kept by its name: moved, turned, stretched,
    /// repainted or made solid or not. False for a name nothing holds.
    pub(crate) fn set(
        &mut self,
        generator: &Generator,
        id: PartId,
        seat: Seat,
        part: Part,
    ) -> bool {
        if !self.held.contains_key(&id) {
            return false;
        }
        let before = self.hold(generator, id, seat, part).expect("held");
        self.mesh(&part);
        // The latest set of a part goes at the cadence: a drag sets it every
        // frame. Alone, sets of one part in a row are one change.
        match &mut self.link {
            Some(link) => {
                link.dirty.insert(id, (seat, part));
            }
            None => {
                self.undone.clear();
                let same = matches!(self.done.last(), Some(Change::Set(last, _, _)) if *last == id);
                if !same {
                    self.done.push(Change::Set(id, before.0, before.1));
                }
            }
        }
        true
    }

    /// Removes a part by its name. False for a name nothing holds.
    pub(crate) fn remove(&mut self, id: PartId) -> bool {
        let Some((seat, part)) = self.let_go(id) else {
            return false;
        };
        match &mut self.link {
            Some(link) => {
                link.dirty.remove(&id);
                self.send(id, Deferred::Remove);
            }
            None => {
                self.undone.clear();
                self.done.push(Change::Removed(id, seat, part));
            }
        }
        true
    }

    /// The world from a part's own space, with the centre of its body in it.
    pub fn transform(&self, generator: &Generator, id: PartId) -> Option<DAffine3> {
        let (seat, part) = self.get(id)?;
        Some(self.place_of(generator, seat, &part.pose))
    }

    /// Where a part in a pose on a seat stands in the world: its box's frame.
    pub(crate) fn place_of(&self, generator: &Generator, seat: Seat, pose: &Pose) -> DAffine3 {
        DAffine3::from_translation(self.centre(seat.body()))
            * seat::seated(seat.sphere(generator), seat.sector(), pose)
    }

    /// The nearest part a line from `from` toward `toward`, unit, meets,
    /// and how far along, metres.
    pub fn pick(&self, generator: &Generator, from: DVec3, toward: DVec3) -> Option<(PartId, f64)> {
        self.hit(generator, from, toward)
            .map(|(id, away_m, _, _)| (id, away_m))
    }

    /// The first part a line of the world meets: which, how far along the
    /// line in metres, which way the side it meets faces in the world, and
    /// which side of the part's own box that is (`+z` its top).
    pub fn hit(
        &self,
        generator: &Generator,
        from: DVec3,
        toward: DVec3,
    ) -> Option<(PartId, f64, DVec3, DVec3)> {
        self.all()
            .filter_map(|(id, seat, part)| {
                // The line in the flat frame of the part's anchor, in blocks.
                let frame = seat::frame(seat.sphere(generator), seat.sector(), part.pose.at);
                let inward = frame.inverse();
                let flat_from = inward.transform_point3(from - self.centre(seat.body()))
                    + DVec3::from(part.pose.at);
                let flat_toward = inward.transform_vector3(toward).normalize();
                let (blocks, side) = part.hit_side(flat_from, flat_toward)?;
                let outward = frame
                    .transform_vector3(part.pose.inward().inverse().transform_vector3(side))
                    .normalize();
                Some((id, blocks * BLOCK_M, outward, side))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
    }

    /// What a body with its feet `feet_m` over the datum, standing in a
    /// column of a seat, has under and over it of the solid parts on that
    /// body. Asked in the world, up the body's own radial, so a part whose
    /// box reaches over a seam holds a body on the next sector all the
    /// same. `None` with none near.
    pub fn footing(
        &self,
        generator: &Generator,
        seat: Seat,
        point: SurfacePoint,
        feet_m: f64,
    ) -> Option<Footing> {
        let body = seat.body();
        let sphere = seat::sphere_of(generator, body);
        let foot = DVec3::from(sphere.position(point, feet_m));
        let tangents = sphere.blocks().tangents(point);
        let up = DVec3::from(tangents.up);
        let half_m = BODY_HALF * BLOCK_M;
        let (du, dv) = (
            DVec3::from(tangents.du).normalize() * half_m,
            DVec3::from(tangents.dv).normalize() * half_m,
        );
        let feet = [DVec3::ZERO, -du - dv, du - dv, -du + dv, du + dv];
        // The parts of the column's bucket, and those over a seam of this
        // body, whichever sector they are seated on.
        let bucket = [
            (point.u / BUCKET).floor() as i32,
            (point.v / BUCKET).floor() as i32,
        ];
        let near = self.buckets.get(&(seat, bucket)).into_iter().flatten();
        near.chain(self.edges.iter())
            .filter_map(|id| self.held.get(id))
            .filter(|(on, part)| on.body() == body && part.solid)
            .filter_map(|(on, part)| {
                // Near enough to be over the body at all: its farthest
                // corner, and the body's width, from the body's column.
                let placed = seat::seated(sphere, on.sector(), &part.pose);
                let to = placed.translation - foot;
                let across = (to - up * to.dot(up)).length();
                if across > part.reach() * BLOCK_M + half_m {
                    return None;
                }
                self.gap_of(part, &placed.inverse(), foot, up, &feet)
            })
            .map(|gap| Footing {
                floor_m: gap.floor.map(|t| feet_m + t),
                ceiling_m: gap.ceiling.map(|t| feet_m + t),
            })
            .reduce(Footing::with)
    }

    /// Every part drawn as a model, placed in the world this frame, and the
    /// marks over them. A part further than `reach_m` from the body is not
    /// drawn. `body` is where the client's own body is, from the centre of
    /// the body it stands on.
    pub fn drawn(
        &self,
        generator: &Generator,
        body: (Body, DVec3),
        reach_m: f64,
        clock_s: f64,
    ) -> Vec<ModelDraw> {
        let mut drawn = Vec::new();
        for (_, seat, part) in self.all() {
            if away_m(generator, seat, &part.pose, body) > reach_m {
                continue;
            }
            let transform = self.place_of(generator, seat, &part.pose);
            // A model whose file is still on its way: its box in glass, and
            // a mark turning over it.
            if let Shape::Vox(hash) = part.shape
                && !self.grids.contains_key(&hash)
            {
                let Some(mesh) = model_id(Shape::Cube, LOADING_PAINT) else {
                    continue;
                };
                drawn.push(ModelDraw {
                    mesh,
                    transform,
                    pass: ModelPass::Glass,
                });
                if let Some(mesh) = model_id(Shape::Cube, WAITING_PAINT) {
                    drawn.push(ModelDraw {
                        mesh,
                        transform: waiting_mark(&transform, clock_s),
                        pass: ModelPass::Over,
                    });
                }
                continue;
            }
            let Some(mesh) = model_of(part) else {
                continue;
            };
            // A landmark and a portal are solid to the eye whatever their
            // paint says.
            let glass = Paint::of(part.paint).finish == Finish::Glass
                && !matches!(part.shape, Shape::Landmark(_) | Shape::Portal(_));
            drawn.push(ModelDraw {
                mesh,
                transform,
                pass: match glass {
                    true => ModelPass::Glass,
                    false => ModelPass::Solid,
                },
            });
        }
        drawn.extend(self.marks.iter().copied());
        drawn
    }

    /// Every part that is a picture within `reach_m` of the body, placed in
    /// the world this frame at the level it is seen at from the screen. A
    /// picture in a light finish is a screen, shining with its own colour;
    /// any other is lit as a print.
    pub(crate) fn pictures(
        &self,
        generator: &Generator,
        body: (Body, DVec3),
        reach_m: f64,
        pictures: &mut Pictures,
        screen: &Screen,
        requests: &mut Requests,
    ) -> Vec<PictureDraw> {
        self.all()
            .filter_map(|(_, seat, part)| {
                let Shape::Image(media) = part.shape else {
                    return None;
                };
                if away_m(generator, seat, &part.pose, body) > reach_m {
                    return None;
                }
                let world_from_quad = self.place_of(generator, seat, &part.pose) * FRONT;
                // As the part says; a picture in the light finish with no
                // shine said is a screen.
                let screen_like = Paint::of(part.paint).finish == Finish::Light;
                let shine = match (part.shine > 0.0, screen_like) {
                    (true, _) => part.shine,
                    (false, true) => 1.0,
                    (false, false) => 0.0,
                };
                Some(pictures.draw(
                    &media,
                    Placed {
                        world_from_quad,
                        shine,
                        reach_m: part.reach_m,
                        both_sides: true,
                        volume: part.loudness(),
                    },
                    screen,
                    requests,
                ))
            })
            .collect()
    }

    /// Every sound part, where it stands, told to `speakers` against where
    /// the eye is: what is within reach plays.
    pub(crate) fn speakers(&self, generator: &Generator, eye: DVec3, speakers: &mut Speakers) {
        for (id, seat, part) in self.all() {
            let Shape::Sound(clip) = part.shape else {
                continue;
            };
            let at = self.place_of(generator, seat, &part.pose).translation;
            speakers.hear(id, &clip, at, part.reach_m, part.loudness(), eye);
        }
    }

    /// The lamps among the parts: a cube in the light finish and a landmark
    /// light what is near, as a cell of that finish does, by their colour,
    /// as many sides as their box has and as much as they shine. A model
    /// has its own colours and a picture its own image, so neither is a
    /// lamp.
    pub fn lamps(&self, generator: &Generator) -> Vec<Lamp> {
        self.all()
            .filter(|(_, _, part)| is_lamp(part))
            .map(|(_, seat, part)| {
                let placed = self.place_of(generator, seat, &part.pose);
                let [sx, sy, sz] = part.pose.scale.map(|n| n as f32);
                // The sides of its box, in a cell's sides.
                let sides = 2.0 * (sx * sy + sy * sz + sz * sx);
                let many = sides.min(LAMP_SIDES).sqrt();
                let shine = if part.shine > 0.0 { part.shine } else { 1.0 };
                let color = glam::Vec3::from(
                    paint_rgb(part.paint).map(|c| (f32::from(c) / 255.0).powf(2.2)),
                );
                let reach_m = match part.reach_m > 0.0 {
                    true => part.reach_m,
                    false => (LAMP_REACH_M + LAMP_GROWS_M * (many - 1.0)).min(LAMP_FAR_M),
                };
                Lamp {
                    position: placed.transform_point3(DVec3::new(0.0, 0.0, 0.5)),
                    color: color * LAMP_GAIN * many * shine,
                    reach_m,
                    // Half its mean side: nearer than that it is a patch, not
                    // a point, and its light fades gently.
                    size_m: (sx * sy * sz).cbrt() * BLOCK_M as f32 / 2.0,
                }
            })
            .collect()
    }

    pub(crate) fn drain_changes(&mut self) -> Vec<ModelChange> {
        core::mem::take(&mut self.changes)
    }

    /// Announces the model of a part, once. A file's is asked for instead,
    /// and announced when it arrives.
    fn mesh(&mut self, part: &Part) {
        self.ensure(part.shape, part.paint);
    }

    /// The model of a shape in a paint, announced the first time it is
    /// asked for. A file's is asked for instead, and announced when it
    /// arrives. `None` for a shape drawn another way, a picture.
    fn ensure(&mut self, shape: Shape, paint: u8) -> Option<ModelId> {
        let id = model_id(shape, paint)?;
        if !self.meshed.contains(&id) {
            match shape {
                Shape::Cube => {
                    self.meshed.push(id);
                    self.changes.push(ModelChange::Add(id, cube(paint)));
                }
                Shape::Landmark(_) => {
                    self.meshed.push(id);
                    self.changes.push(ModelChange::Add(id, star(paint)));
                }
                Shape::Portal(portal) => {
                    self.meshed.push(id);
                    self.changes
                        .push(ModelChange::Add(id, ring(paint, portal.rim_of_width())));
                }
                Shape::Vox(hash) => {
                    // Until the file arrives, its place is a glass box with
                    // a mark turning over it.
                    self.ensure(Shape::Cube, LOADING_PAINT);
                    self.ensure(Shape::Cube, WAITING_PAINT);
                    self.want(hash);
                }
                // A picture is no model: drawn by `pictures`. A sound is
                // nothing to the eye.
                Shape::Image(_) | Shape::Sound(_) => {}
            }
        }
        Some(id)
    }
    /// Asks for a model's file, once: what a hand wants before it places
    /// the model, to know its size.
    pub(crate) fn want(&mut self, hash: [u8; 32]) {
        if self.grids.contains_key(&hash) || self.asked.contains(&hash) {
            return;
        }
        self.asked.push(hash);
        self.wanted.push(hash);
    }

    /// The files asked for since last taken.
    pub(crate) fn wanted(&mut self) -> Vec<[u8; 32]> {
        core::mem::take(&mut self.wanted)
    }

    /// A model's file arrived: its grid is kept and its mesh announced, the
    /// voxels filling the unit box.
    pub(crate) fn model_arrived(&mut self, hash: [u8; 32], model: vox::Model) {
        let id = model_of_file(hash);
        if !self.meshed.contains(&id) {
            self.meshed.push(id);
            self.changes
                .push(ModelChange::Add(id, boxed(&model.grid, &model.palette)));
        }
        self.grids.insert(hash, model.grid);
    }

    /// How many voxels a model of a file has along each axis, once it
    /// arrived.
    pub fn size_of(&self, hash: [u8; 32]) -> Option<[u32; 3]> {
        self.grids.get(&hash).map(|grid| grid.size)
    }

    /// What a body standing at `foot` in the world has under and over it of
    /// one part, along `up`, as `part::gap_along` says it: its box, and for
    /// a model whose file arrived, its own voxels. `inward` is the part's
    /// own space from the world.
    fn gap_of(
        &self,
        part: &Part,
        inward: &DAffine3,
        foot: DVec3,
        up: DVec3,
        feet: &[DVec3],
    ) -> Option<Gap> {
        let boxed = part::gap_along(inward, foot, up, feet, STEP_M)?;
        let Shape::Vox(hash) = part.shape else {
            return Some(boxed);
        };
        match self.grids.get(&hash) {
            Some(grid) => model_gap(grid, inward, foot, up, feet, STEP_M),
            None => Some(boxed),
        }
    }
}

/// The mark over a loading part: a bar over the middle of the top of its
/// box, turning about the part's own up with the clock. `world` is the
/// world from the part's own space.
fn waiting_mark(world: &DAffine3, clock_s: f64) -> DAffine3 {
    let up = world.z_axis.normalize();
    let top = world.transform_point3(DVec3::new(0.0, 0.0, 1.0)) + up * WAITING_OVER_M;
    let turn =
        glam::DQuat::from_axis_angle(up, clock_s * WAITING_TURNS_HZ * core::f64::consts::TAU);
    let x = turn * world.x_axis.normalize();
    let y = up.cross(x);
    let frame = glam::DMat3::from_cols(x, y, up);
    // The bar lies along x, centred on the top: the unit box rises from
    // its anchor, so it is lowered by half its thickness.
    DAffine3::from_mat3_translation(frame, top - up * (WAITING_THICK_M / 2.0))
        * DAffine3::from_translation(DVec3::new(0.0, 0.0, 0.0))
        * DAffine3::from_scale(DVec3::new(WAITING_BAR_M, WAITING_THICK_M, WAITING_THICK_M))
}

/// How far a body at `body`, from the centre of the body it is on, is from
/// a part's anchor, metres: infinite from another body.
pub(crate) fn away_m(generator: &Generator, seat: Seat, pose: &Pose, body: (Body, DVec3)) -> f64 {
    if seat.body() != body.0 {
        return f64::INFINITY;
    }
    let anchor = seat::seated(seat.sphere(generator), seat.sector(), pose).translation;
    anchor.distance(body.1)
}

/// Whether a part is a lamp, lighting what is near: a cube in the light
/// finish, and a landmark and a portal's rim whatever their finish, since
/// they are lit with their own light to be seen at night. A model has its
/// own colours and a picture its own image.
pub fn is_lamp(part: &Part) -> bool {
    match part.shape {
        Shape::Cube => Paint::of(part.paint).finish == Finish::Light,
        Shape::Landmark(_) => true,
        // A portal with no rim has nothing to shine.
        Shape::Portal(portal) => portal.rim > 0,
        Shape::Vox(_) | Shape::Image(_) | Shape::Sound(_) => false,
    }
}

/// The model a part is drawn as. `None` for a shape drawn another way, a
/// picture.
fn model_of(part: &Part) -> Option<ModelId> {
    model_id(part.shape, part.paint)
}

/// The model of one of the core's shapes in a paint, or of a file by its
/// hash.
fn model_id(shape: Shape, paint: u8) -> Option<ModelId> {
    // A landmark's name and a portal's destination are not in their
    // models: every landmark in a paint shares one star, every portal with
    // a rim of one width one ring. A portal with no rim has no model: its
    // pane is drawn on its own.
    let shape = match shape {
        Shape::Cube => 0,
        Shape::Landmark(_) => 1,
        Shape::Portal(Portal { rim: 0, .. }) => return None,
        Shape::Portal(portal) => RINGS | u64::from(portal.rim),
        Shape::Vox(hash) => return Some(model_of_file(hash)),
        Shape::Image(_) | Shape::Sound(_) => return None,
    };
    debug_assert!(shape < SHAPES >> 8);
    Some(ModelId((shape << 8) | u64::from(paint)))
}

fn model_of_file(hash: [u8; 32]) -> ModelId {
    let head: [u8; 8] = hash[..8].try_into().expect("eight of thirty two");
    ModelId(u64::from_be_bytes(head) | FILE)
}

/// What a body standing at `foot` has under its feet and over its head of
/// a model's own voxels, along `up`, read through the part's pose: up each
/// foot's line through the part's box, the solid runs of the grid along it.
/// Solid at the height of the step is a wall whose top is the floor
/// offered, below it a floor, above it a roof, as a box answers
/// (`part::gap_along`), in the same frame and units.
fn model_gap(
    grid: &vox::Grid,
    inward: &DAffine3,
    foot: DVec3,
    up: DVec3,
    feet: &[DVec3],
    reach: f64,
) -> Option<Gap> {
    let up_in = inward.transform_vector3(up);
    let size = grid.size.map(f64::from);
    let to_grid = |p: DVec3| [(p.x + 0.5) * size[0], (p.y + 0.5) * size[1], p.z * size[2]];
    let mut gap: Option<Gap> = None;
    for &at in feet {
        let foot_in = inward.transform_point3(foot + at);
        let Some((low, high)) = part::through(foot_in, up_in) else {
            continue;
        };
        let runs = grid.solid_runs(
            to_grid(foot_in + up_in * low),
            to_grid(foot_in + up_in * high),
        );
        let mut found = Gap {
            floor: None,
            ceiling: None,
        };
        for (a, b) in runs {
            let (h0, h1) = (low + a * (high - low), low + b * (high - low));
            if h0 <= reach && reach <= h1 || h1 <= reach {
                found.floor = Some(found.floor.map_or(h1, |floor| floor.max(h1)));
            } else {
                found.ceiling = Some(found.ceiling.map_or(h0, |roof| roof.min(h0)));
            }
        }
        if found.floor.is_none() && found.ceiling.is_none() {
            continue;
        }
        gap = Some(match gap {
            None => found,
            Some(gap) => Gap {
                floor: pick(gap.floor, found.floor, f64::max),
                ceiling: pick(gap.ceiling, found.ceiling, f64::min),
            },
        });
    }
    gap
}

fn pick(a: Option<f64>, b: Option<f64>, f: fn(f64, f64) -> f64) -> Option<f64> {
    match (a, b) {
        (Some(a), Some(b)) => Some(f(a, b)),
        (a, b) => a.or(b),
    }
}

/// A model's voxels meshed into the unit box: `vox` meshes them in voxels
/// from the middle of the footprint, and here each axis is divided by how
/// many voxels it has. Every colour is the file's own, and no side has an
/// edge drawn.
fn boxed(grid: &vox::Grid, palette: &vox::Palette) -> ModelMesh {
    let mesh = vox::mesh(grid, palette);
    let size = grid.size.map(|n| n as f32);
    let across = [[0, 0], [u8::MAX, 0], [u8::MAX, u8::MAX], [0, u8::MAX]];
    let vertices = mesh
        .vertices
        .iter()
        .enumerate()
        .map(|(k, v)| {
            let [u, w] = across[k % 4];
            VolumeVertex {
                position: [0, 1, 2].map(|i| v.position[i] / size[i]),
                normal: v.normal,
                color: [v.color[0], v.color[1], v.color[2], 0],
                side: [v.sky, u, w, 0],
            }
        })
        .collect();
    ModelMesh {
        vertices,
        indices: mesh.indices,
    }
}
/// The front of a part's box as the picture's quad: the unit quad, `x`
/// across and `y` up, facing `+z`, laid on the box's `y = -0.5` side with
/// its bottom at the anchor, facing `-y`.
const FRONT: DAffine3 = DAffine3 {
    matrix3: DMat3::from_cols(DVec3::X, DVec3::Z, DVec3::NEG_Y),
    translation: DVec3::new(0.0, -0.5, 0.5),
};

/// A cube of one voxel in a paint: the unit box, its sides wound counter
/// clockwise seen from outside, every corner open to the sky.
fn cube(paint: u8) -> ModelMesh {
    let (color, edge) = paint_look(paint);
    let mut mesh = ModelMesh {
        vertices: Vec::with_capacity(24),
        indices: Vec::with_capacity(36),
    };
    block(&mut mesh, [-0.5, -0.5, 0.0], [0.5, 0.5, 1.0], color, edge);
    mesh
}

/// Adds a block between two corners to a mesh: six sides wound counter
/// clockwise seen from outside, every corner open to the sky.
fn block(mesh: &mut ModelMesh, min: [f32; 3], max: [f32; 3], color: [u8; 4], edge: u8) {
    // Each side from its first corner: across, then up, then back, each
    // corner as which end of the box it is on, along each axis.
    let sides: [([f32; 3], [[usize; 3]; 4]); 6] = [
        (
            [0.0, 0.0, 1.0],
            [[0, 0, 1], [1, 0, 1], [1, 1, 1], [0, 1, 1]],
        ),
        (
            [0.0, 0.0, -1.0],
            [[0, 0, 0], [0, 1, 0], [1, 1, 0], [1, 0, 0]],
        ),
        (
            [1.0, 0.0, 0.0],
            [[1, 0, 0], [1, 1, 0], [1, 1, 1], [1, 0, 1]],
        ),
        (
            [-1.0, 0.0, 0.0],
            [[0, 0, 0], [0, 0, 1], [0, 1, 1], [0, 1, 0]],
        ),
        (
            [0.0, 1.0, 0.0],
            [[0, 1, 0], [0, 1, 1], [1, 1, 1], [1, 1, 0]],
        ),
        (
            [0.0, -1.0, 0.0],
            [[0, 0, 0], [1, 0, 0], [1, 0, 1], [0, 0, 1]],
        ),
    ];
    let ends = [min, max];
    let across = [[0, 0], [u8::MAX, 0], [u8::MAX, u8::MAX], [0, u8::MAX]];
    for (normal, corners) in sides {
        let base = mesh.vertices.len() as u32;
        for (corner, [u, v]) in corners.into_iter().zip(across) {
            let position = [0, 1, 2].map(|axis| ends[corner[axis]][axis]);
            mesh.vertices.push(VolumeVertex {
                position,
                normal,
                color,
                side: [u8::MAX, u, v, edge],
            });
        }
        mesh.indices.extend([0, 1, 2, 0, 2, 3].map(|k| base + k));
    }
}

/// How many segments a portal's ring is drawn in. The pane inside it is
/// the renderer's ([`Parts::panes`]): a window on the world seen through
/// the portal.
const RING_SEGMENTS: usize = 32;

/// A portal's ring in a paint: a rim `rim` of the box's width wide and as
/// thick as the box along `y`, round the circle that fills the box along
/// `x` and `z`. Each face of the rim is a quad a segment, flat, wound
/// counter clockwise seen from where it faces. The rim shines with its own
/// light whatever the paint's finish, so a portal is seen at night.
fn ring(paint: u8, rim: f32) -> ModelMesh {
    let (mut color, edge) = paint_look(paint);
    color[3] = u8::MAX;
    let mut mesh = ModelMesh {
        vertices: Vec::with_capacity(RING_SEGMENTS * 16),
        indices: Vec::with_capacity(RING_SEGMENTS * 24),
    };
    let centre = [0.0, 0.5];
    let (outer, inner) = (0.5, 0.5 - rim);
    let round = |k: usize, radius: f32| -> [f32; 2] {
        let angle = k as f32 * core::f32::consts::TAU / RING_SEGMENTS as f32;
        [
            centre[0] + radius * angle.cos(),
            centre[1] + radius * angle.sin(),
        ]
    };
    let across = [[0, 0], [u8::MAX, 0], [u8::MAX, u8::MAX], [0, u8::MAX]];
    let mut quad = |corners: [[f32; 3]; 4], normal: [f32; 3], color: [u8; 4], edge: u8| {
        let base = mesh.vertices.len() as u32;
        for (position, [u, v]) in corners.into_iter().zip(across) {
            mesh.vertices.push(VolumeVertex {
                position,
                normal,
                color,
                side: [u8::MAX, u, v, edge],
            });
        }
        mesh.indices.extend([0, 1, 2, 0, 2, 3].map(|k| base + k));
    };
    for k in 0..RING_SEGMENTS {
        let (a, b) = (round(k, outer), round(k + 1, outer));
        let (c, d) = (round(k, inner), round(k + 1, inner));
        let at = |[x, z]: [f32; 2], y: f32| [x, y, z];
        // The front, facing `-y`, and the back: a quad of the annulus each.
        quad(
            [at(a, -0.5), at(b, -0.5), at(d, -0.5), at(c, -0.5)],
            [0.0, -1.0, 0.0],
            color,
            edge,
        );
        quad(
            [at(a, 0.5), at(c, 0.5), at(d, 0.5), at(b, 0.5)],
            [0.0, 1.0, 0.0],
            color,
            edge,
        );
        // The outside and the inside of the rim, their normals radial, flat
        // over the segment.
        let mid = round(k, outer);
        let next = round(k + 1, outer);
        let radial = [
            (mid[0] + next[0]) / 2.0 - centre[0],
            (mid[1] + next[1]) / 2.0 - centre[1],
        ];
        let length = (radial[0] * radial[0] + radial[1] * radial[1]).sqrt();
        let out = [radial[0] / length, 0.0, radial[1] / length];
        quad(
            [at(a, -0.5), at(a, 0.5), at(b, 0.5), at(b, -0.5)],
            out,
            color,
            edge,
        );
        quad(
            [at(c, -0.5), at(d, -0.5), at(d, 0.5), at(c, 0.5)],
            [-out[0], 0.0, -out[2]],
            color,
            edge,
        );
    }
    mesh
}

/// How many points a landmark's star has, and how far in its inner corners
/// stand, of the box's half width.
const STAR_POINTS: usize = 5;
const STAR_INNER: f32 = 0.38;

/// A landmark's star in a paint: a prism of a five pointed star filling the
/// unit box, a point toward `+y`, its top and bottom fans and a side a rim
/// edge, wound counter clockwise seen from outside, every corner open to
/// the sky. It shines with its own light whatever the paint's finish.
fn star(paint: u8) -> ModelMesh {
    let (mut color, _) = paint_look(paint);
    color[3] = u8::MAX;
    let rim: Vec<[f32; 2]> = (0..STAR_POINTS * 2)
        .map(|k| {
            let angle = core::f32::consts::FRAC_PI_2
                + k as f32 * core::f32::consts::PI / STAR_POINTS as f32;
            let radius = if k % 2 == 0 { 0.5 } else { 0.5 * STAR_INNER };
            [radius * angle.cos(), radius * angle.sin()]
        })
        .collect();
    let mut vertices = Vec::with_capacity(2 * (rim.len() + 1) + 4 * rim.len());
    let mut indices = Vec::with_capacity(6 * rim.len() + 6 * rim.len());
    let vertex = |position: [f32; 3], normal: [f32; 3], across: [u8; 2]| VolumeVertex {
        position,
        normal,
        color,
        side: [u8::MAX, across[0], across[1], 0],
    };
    // The top and the bottom: a fan from the middle round the rim, the top
    // seen from above and the bottom from below.
    for (z, normal, forward) in [(1.0, [0.0, 0.0, 1.0], true), (0.0, [0.0, 0.0, -1.0], false)] {
        let base = vertices.len() as u32;
        vertices.push(vertex([0.0, 0.0, z], normal, [0, 0]));
        for &[x, y] in &rim {
            vertices.push(vertex([x, y, z], normal, [u8::MAX, u8::MAX]));
        }
        let n = rim.len() as u32;
        for k in 0..n {
            let (a, b) = (base + 1 + k, base + 1 + (k + 1) % n);
            match forward {
                true => indices.extend([base, a, b]),
                false => indices.extend([base, b, a]),
            }
        }
    }
    // The sides: a quad an edge of the rim, facing out of it.
    let across = [[0, 0], [u8::MAX, 0], [u8::MAX, u8::MAX], [0, u8::MAX]];
    for k in 0..rim.len() {
        let (a, b) = (rim[k], rim[(k + 1) % rim.len()]);
        let d = [b[0] - a[0], b[1] - a[1]];
        let length = (d[0] * d[0] + d[1] * d[1]).sqrt();
        let normal = [d[1] / length, -d[0] / length, 0.0];
        let base = vertices.len() as u32;
        let corners = [
            [a[0], a[1], 0.0],
            [b[0], b[1], 0.0],
            [b[0], b[1], 1.0],
            [a[0], a[1], 1.0],
        ];
        for (position, uv) in corners.into_iter().zip(across) {
            vertices.push(vertex(position, normal, uv));
        }
        indices.extend([0, 1, 2, 0, 2, 3].map(|k| base + k));
    }
    ModelMesh { vertices, indices }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    #[test]
    fn a_cube_is_six_sides_wound_outward() {
        let mesh = cube(5);
        assert_eq!((mesh.vertices.len(), mesh.indices.len()), (24, 36));
        for side in mesh.indices.chunks(3) {
            let [a, b, c] = [0, 1, 2].map(|i| Vec3::from(mesh.vertices[side[i] as usize].position));
            let normal = Vec3::from(mesh.vertices[side[0] as usize].normal);
            let wound = (b - a).cross(c - a);
            assert!(wound.dot(normal) > 0.0, "{side:?} winds against its normal");
            // Every corner is on the side its normal names.
            let centre = Vec3::new(0.0, 0.0, 0.5);
            assert!(((a - centre).dot(normal) - 0.5).abs() < 1e-6);
        }
    }

    #[test]
    fn a_star_is_wound_outward_and_shines_whatever_its_paint() {
        let mesh = star(5);
        assert_eq!((mesh.vertices.len(), mesh.indices.len()), (62, 120));
        for side in mesh.indices.chunks(3) {
            let [a, b, c] = [0, 1, 2].map(|i| Vec3::from(mesh.vertices[side[i] as usize].position));
            let normal = Vec3::from(mesh.vertices[side[0] as usize].normal);
            let wound = (b - a).cross(c - a);
            assert!(wound.dot(normal) > 0.0, "{side:?} winds against its normal");
            // Every corner is in the unit box, and a side's corners are on
            // the plane its normal names.
            for p in [a, b, c] {
                assert!(p.x.abs() <= 0.5 + 1e-6 && p.y.abs() <= 0.5 + 1e-6);
                assert!((0.0..=1.0).contains(&p.z));
                assert!(((p - a).dot(normal)).abs() < 1e-6);
            }
        }
        assert!(mesh.vertices.iter().all(|v| v.color[3] == u8::MAX));
        // A point toward +y reaches the box's side.
        assert!(
            mesh.vertices
                .iter()
                .any(|v| (v.position[1] - 0.5).abs() < 1e-6)
        );
        let name = part::Name::new("here").unwrap();
        assert_eq!(
            model_id(Shape::Landmark(name), 5),
            model_id(Shape::Landmark(part::Name::new("there").unwrap()), 5),
            "every landmark in a paint shares one star"
        );
        assert_ne!(model_id(Shape::Landmark(name), 5), model_id(Shape::Cube, 5));
    }

    #[test]
    fn a_ring_is_a_rim_round_an_open_pane() {
        let rim = 0.06;
        let mesh = ring(5, rim);
        let n = RING_SEGMENTS;
        assert_eq!((mesh.vertices.len(), mesh.indices.len()), (16 * n, 24 * n));
        for side in mesh.indices.chunks(3) {
            let [a, b, c] = [0, 1, 2].map(|i| Vec3::from(mesh.vertices[side[i] as usize].position));
            let normal = Vec3::from(mesh.vertices[side[0] as usize].normal);
            let wound = (b - a).cross(c - a);
            assert!(wound.dot(normal) > 0.0, "{side:?} winds against its normal");
            for p in [a, b, c] {
                assert!(p.x.abs() <= 0.5 + 1e-6 && p.y.abs() <= 0.5 + 1e-6);
                assert!((-1e-6..=1.0 + 1e-6).contains(&p.z));
                assert!(((p - a).dot(normal)).abs() < 1e-5);
            }
        }
        // The rim fills the box: a corner reaches each of its sides.
        for (axis, end) in [(0, -0.5), (0, 0.5), (2, 0.0), (2, 1.0)] {
            assert!(
                mesh.vertices
                    .iter()
                    .any(|v| (v.position[axis] - end).abs() < 1e-6)
            );
        }
        // The rim is the paint's and leaves the pane open inside it.
        assert!(mesh.vertices.iter().all(|v| {
            let [x, _, z] = v.position;
            (x * x + (z - 0.5) * (z - 0.5)).sqrt() >= 0.5 - rim - 1e-6
        }));
        assert!(
            mesh.vertices.iter().all(|v| v.color[3] == u8::MAX),
            "the rim shines whatever its paint"
        );
        let portal = |to: Option<PartId>, rim: u8| Shape::Portal(Portal { to, rim });
        assert_eq!(
            model_id(portal(Some(PartId(1)), 12), 5),
            model_id(portal(None, 12), 5),
            "every portal in a paint with a rim of one width shares one ring"
        );
        assert_ne!(model_id(portal(None, 12), 5), model_id(portal(None, 13), 5));
        assert_ne!(model_id(portal(None, 12), 5), model_id(Shape::Cube, 5));
        assert_eq!(model_id(portal(None, 0), 5), None, "no rim, no ring");
        assert!(!is_lamp(&Part {
            shape: portal(None, 0),
            pose: Pose::upright([0.0; 3], [4.0, 0.1, 4.0]),
            paint: 5,
            solid: false,
            shine: 0.0,
            reach_m: 0.0,
            name: None,
            muted: false,
            volume: 0.0,
        }));
        // The cube is still the unit box.
        let cube = cube(5);
        assert_eq!((cube.vertices.len(), cube.indices.len()), (24, 36));
        assert!(cube.vertices.iter().all(|v| {
            let [x, y, z] = v.position;
            x.abs() == 0.5 && y.abs() == 0.5 && (z == 0.0 || z == 1.0)
        }));
    }

    #[test]
    fn a_part_over_a_seam_holds_a_body_on_the_next_sector() {
        let generator = Generator::new(worldgen::Recipe::new(1)).unwrap();
        let sphere = generator.sphere();
        let side = f64::from(sphere.blocks().side());
        let mut parts = Parts::default();
        let seat = Seat::Sector(topology::Sector::new(0).unwrap());
        // Anchored half a block inside the edge, four blocks wide: its box
        // reaches a block and a half over the seam.
        let part = Part {
            shape: Shape::Cube,
            pose: Pose::upright([side - 0.5, side / 2.0, 20.0], [4.0, 4.0, 2.0]),
            paint: 0,
            solid: true,
            shine: 0.0,
            reach_m: 0.0,
            name: None,
            muted: false,
            volume: 0.0,
        };
        parts.place(&generator, seat, part);
        // A body a block past the seam, on the next sector, at the top.
        let placed = seat::seated(sphere, seat.sector(), &part.pose);
        let over = placed.transform_point3(DVec3::new(0.375, 0.0, 1.0));
        let point = sphere.blocks().surface_point(over.to_array());
        assert_ne!(point.sector, seat.sector(), "the body is over the seam");
        let feet_m = over.length() - sphere.radius_m();
        let footing = parts
            .footing(&generator, Seat::Sector(point.sector), point, feet_m)
            .unwrap();
        assert!(
            (footing.floor_m.unwrap() - feet_m).abs() < 1e-3,
            "{footing:?} vs {feet_m}"
        );
    }

    #[test]
    fn a_body_stands_on_a_solid_cube_and_not_on_an_open_one() {
        let generator = Generator::new(worldgen::Recipe::new(1)).unwrap();
        let mut parts = Parts::default();
        let seat = Seat::Sector(topology::Sector::new(0).unwrap());
        let at = [1000.0, 1000.0, 10.0];
        let part = Part {
            shape: Shape::Cube,
            pose: Pose::upright(at, CUBE),
            paint: 0,
            solid: true,
            shine: 0.0,
            reach_m: 0.0,
            name: None,
            muted: false,
            volume: 0.0,
        };
        let id = parts.place(&generator, seat, part);
        let point = SurfacePoint::new(seat.sector(), 1000.3, 999.8);
        let top_m = 12.0 * BLOCK_M;
        let footing = parts.footing(&generator, seat, point, top_m).unwrap();
        assert!(
            (footing.floor_m.unwrap() - top_m).abs() < 1e-4,
            "{footing:?}"
        );
        // Under it, a roof.
        let under = parts.footing(&generator, seat, point, 0.0).unwrap();
        assert!(
            (under.ceiling_m.unwrap() - 10.0 * BLOCK_M).abs() < 1e-4,
            "{under:?}"
        );
        // A step away, nothing.
        let aside = SurfacePoint::new(seat.sector(), 1003.0, 1000.0);
        assert!(parts.footing(&generator, seat, aside, top_m).is_none());
        // Made open, nothing to stand on.
        parts.set(
            &generator,
            id,
            seat,
            Part {
                solid: false,
                ..part
            },
        );
        assert!(parts.footing(&generator, seat, point, top_m).is_none());
        // Aimed at from above in the world, it is met at its top.
        let world = parts.transform(&generator, id).unwrap();
        let top = world.transform_point3(DVec3::new(0.0, 0.0, 1.0));
        let up = top.normalize();
        let (hit, away_m) = parts.pick(&generator, top + up * 3.0, -up).unwrap();
        assert_eq!(hit, id);
        assert!((away_m - 3.0).abs() < 1e-6, "{away_m}");
        // One model for every cube in a paint.
        parts.place(&generator, seat, part);
        assert_eq!(parts.drain_changes().len(), 1);
    }

    #[test]
    fn a_model_is_asked_for_once_and_stood_on_by_its_own_voxels() {
        // A shelter of four voxels across and four up: a pillar at one end
        // and a roof over air wide and high enough for a body.
        let grid = vox::Grid {
            size: [4, 1, 4],
            voxels: vec![1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1, 1, 1, 1],
        };
        let model = vox::Model {
            grid,
            palette: vox::default_palette(),
        };
        let generator = Generator::new(worldgen::Recipe::new(1)).unwrap();
        let hash = [7; 32];
        let mut parts = Parts::default();
        let seat = Seat::Sector(topology::Sector::new(0).unwrap());
        // A block a voxel: the box is 4 by 1 by 4 blocks, from 998 to 1002
        // across and 10 to 14 up.
        let part = Part {
            shape: Shape::Vox(hash),
            pose: Pose::upright([1000.0, 1000.0, 10.0], [4.0, 1.0, 4.0]),
            paint: 0,
            solid: true,
            shine: 0.0,
            reach_m: 0.0,
            name: None,
            muted: false,
            volume: 0.0,
        };
        parts.place(&generator, seat, part);
        parts.place(&generator, seat, part);
        assert_eq!(parts.wanted(), vec![hash]);
        assert!(parts.wanted().is_empty());
        // Before the file comes, the place it will fill is drawn: a glass
        // box and the mark turning over it, one mesh each.
        assert_eq!(parts.drain_changes().len(), 2);
        // Until it comes, the box holds a body up: on top of it.
        let near = |a: Option<f64>, b: f64| a.is_some_and(|a| (a - b).abs() < 1e-4);
        let under_arch = SurfacePoint::new(seat.sector(), 1000.6, 1000.0);
        let boxed = parts
            .footing(&generator, seat, under_arch, 14.0 * BLOCK_M)
            .unwrap();
        assert!(near(boxed.floor_m, 14.0 * BLOCK_M), "{boxed:?}");

        parts.drain_changes();
        parts.model_arrived(hash, model);
        assert_eq!(parts.size_of(hash), Some([4, 1, 4]));
        // Its mesh; the glass box and the mark of its loading were
        // announced before.
        let changes = parts.drain_changes();
        assert_eq!(changes.len(), 1);
        let mesh = changes
            .iter()
            .find_map(|change| match change {
                ModelChange::Add(id, mesh) if *id == model_of_file(hash) => Some(mesh),
                _ => None,
            })
            .expect("the model's mesh is added");
        // Seven voxels, of which some sides touch: fewer than 42 sides, all
        // inside the unit box.
        assert!(mesh.vertices.len() < 42 * 4 && !mesh.vertices.is_empty());
        for v in &mesh.vertices {
            assert!(v.position[0].abs() <= 0.5 && v.position[1].abs() <= 0.5);
            assert!((0.0..=1.0).contains(&v.position[2]));
        }
        // Standing on top: the top of the roof, four blocks up.
        let on_top = parts
            .footing(&generator, seat, under_arch, 14.0 * BLOCK_M)
            .unwrap();
        assert!(near(on_top.floor_m, 14.0 * BLOCK_M), "{on_top:?}");
        // Under the roof, where the box said floor, the voxels say a roof
        // three blocks over the anchor and no floor of the part.
        let feet_m = 10.0 * BLOCK_M;
        let under = parts.footing(&generator, seat, under_arch, feet_m).unwrap();
        assert_eq!(under.floor_m, None, "{under:?}");
        assert!(near(under.ceiling_m, 13.0 * BLOCK_M), "{under:?}");
        // Over the pillar, feet at the anchor: a wall, whose top is the
        // floor offered.
        let pillar = SurfacePoint::new(seat.sector(), 998.5, 1000.0);
        let wall = parts.footing(&generator, seat, pillar, feet_m).unwrap();
        assert!(near(wall.floor_m, 14.0 * BLOCK_M), "{wall:?}");
    }
}
