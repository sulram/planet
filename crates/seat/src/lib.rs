//! How cells are seated on a world, as the client and the server both hold
//! it (DECISIONS 107, 110): the plots a sector is cut into, where a volume
//! starts and ends by the ground under its plot, a gesture as the wire says
//! it, and how far from a body a volume is held.
//!
//! A rule here is written once and run on both sides: the client predicts
//! with it, and the server decides with it, in its module.

use protocol::Body;
use protocol::cells as wire;
use topology::{BLOCK_M, QuadSphere, Sector, SurfacePoint};
use voxel::{Gesture, Span};
use worldgen::Generator;

/// Blocks along the side of a plot, as a power of two: 64 of them, 32 metres
/// at the middle of a sector. The address of a column, less these bits, is
/// the plot it is on.
pub const PLOT_BITS: u32 = 6;
/// Cells a volume of the planet rises over the highest ground of its plot:
/// 512 metres, a tower's worth and more. A volume is stored by the chunk, so
/// the air in it costs nothing.
pub const HEIGHT: i32 = 1024;
/// Cells a volume of the moon rises: 128 metres, low over a small body.
pub const MOON_HEIGHT: i32 = 256;

/// Cells a volume of a body rises over the highest ground of its plot.
pub fn height(body: Body) -> i32 {
    match body {
        Body::Moon => MOON_HEIGHT,
        Body::Planet => HEIGHT,
    }
}
/// The most cells the box of one change holds: what is read of it to take
/// it back, on both sides.
pub const CHANGE_CELLS: u64 = 1 << 23;
/// Blocks between the columns whose ground is read to find how low and how
/// high the ground of a plot stands.
const SURVEY: i32 = 8;
/// Cells a volume holds under the lowest ground its survey found: the dip
/// between two columns of it.
const UNDER: i32 = 8;

/// How near a body a volume is for its client to hold it, metres: within
/// this the world answers a look with it.
pub const HOLD_M: f64 = 256.0;
/// How far a body goes from a volume before its client lets it go.
pub const DROP_M: f64 = 320.0;
/// How near a body a volume is for its client to hold it as it is seen from
/// afar, metres: past [`HOLD_M`] and within this, the world answers a look
/// with it so. Further than the horizon from a hill of the planet.
pub const AFAR_M: f64 = 2048.0;
/// How far a body goes from a volume held afar before its client lets it go.
pub const AFAR_DROP_M: f64 = 2304.0;
/// How near a body a change is told to its session: further than a client
/// holds a volume, so a body at the edge of holding one hears of it though
/// its stance is a moment old.
pub const TELL_M: f64 = 384.0;

/// What cells are seated on: the frame their addresses are counted in. A
/// sector of the planet, or a sector of the moon, which moves: its cells
/// are counted on the moon's own grid, from the moon's centre, and ride its
/// orbit. A ship with a room aboard is the next kind (DECISIONS 109), so
/// every word of the cells says where as a seat and never as a bare address.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[non_exhaustive]
pub enum Seat {
    Sector(Sector),
    Moon(Sector),
}

impl From<Sector> for Seat {
    fn from(sector: Sector) -> Seat {
        Seat::Sector(sector)
    }
}

/// What is added to a sector's number to name a seat of the moon in a key.
const MOON_KEY: u8 = 16;

impl Seat {
    /// The seat a sector of a body is.
    pub fn on(body: Body, sector: Sector) -> Seat {
        match body {
            Body::Moon => Seat::Moon(sector),
            _ => Seat::Sector(sector),
        }
    }

    /// The sector of its body it is.
    pub fn sector(self) -> Sector {
        let (Seat::Sector(sector) | Seat::Moon(sector)) = self;
        sector
    }

    /// The body it is a sector of.
    pub fn body(self) -> Body {
        match self {
            Seat::Sector(_) => Body::Planet,
            Seat::Moon(_) => Body::Moon,
        }
    }

    /// The body it is on, as its cells are counted: the grid they are cut
    /// by, and the radius that turns them into metres from its centre.
    pub fn sphere(self, generator: &Generator) -> QuadSphere {
        sphere_of(generator, self.body())
    }

    /// The byte that names it in a key of a store.
    pub fn key(self) -> u8 {
        match self {
            Seat::Sector(sector) => sector.index() as u8,
            Seat::Moon(sector) => MOON_KEY + sector.index() as u8,
        }
    }

    /// The seat a byte of a key names.
    pub fn from_key(key: u8) -> Option<Seat> {
        match key.checked_sub(MOON_KEY) {
            Some(sector) => Sector::new(sector).map(Seat::Moon),
            None => Sector::new(key).map(Seat::Sector),
        }
    }

    pub fn wire(self) -> wire::Seat {
        let index = self.sector().index() as u32;
        let seat = match self {
            Seat::Sector(_) => wire::seat::Seat::Sector(index),
            Seat::Moon(_) => wire::seat::Seat::Moon(index),
        };
        wire::Seat { seat: Some(seat) }
    }

    /// The seat a message names, when it names one this version knows.
    pub fn from_wire(seat: Option<&wire::Seat>) -> Option<Seat> {
        let sector = |index: u32| Sector::new(u8::try_from(index).ok()?);
        match seat?.seat? {
            wire::seat::Seat::Sector(index) => sector(index).map(Seat::Sector),
            wire::seat::Seat::Moon(index) => sector(index).map(Seat::Moon),
        }
    }
}

/// A body as cells are counted on it: the grid they are cut by, and the
/// radius that turns them into metres from its centre.
pub fn sphere_of(generator: &Generator, body: Body) -> QuadSphere {
    match body {
        Body::Moon => generator.moon(),
        Body::Planet => generator.sphere(),
    }
}

/// How high the ground of a body stands along a direction from its centre,
/// metres over its datum, in full detail.
pub fn ground_m(generator: &Generator, body: Body, direction: [f64; 3]) -> f64 {
    let sample = match body {
        Body::Moon => generator.moon_sample_at(direction, 0.0),
        Body::Planet => generator.sample_at(direction, 0.0),
    };
    sample.height_m
}

/// Why no volume is seated on a plot.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Unseated {
    /// The ground here is under the sea.
    Sea,
    /// The plot is on the edge of a sector: a volume stays inside one.
    Seam,
}

impl Unseated {
    /// The code a refusal carries on the wire.
    pub fn code(self) -> &'static str {
        match self {
            Unseated::Sea => "sea",
            Unseated::Seam => "seam",
        }
    }
}

/// The plot a column is on: its address, less the bits of a plot.
pub fn plot_of(point: SurfacePoint) -> [i32; 2] {
    [point.u, point.v].map(|at| at.floor() as i32 >> PLOT_BITS)
}

/// The column in the middle of a plot.
pub fn middle(sector: Sector, plot: [i32; 2]) -> SurfacePoint {
    let half = f64::from(1 << (PLOT_BITS - 1));
    let [u, v] = plot.map(|n| f64::from(n << PLOT_BITS) + half);
    SurfacePoint::new(sector, u, v)
}

/// How high the ground of a seat stands at a column of it, in blocks: the
/// ground in full detail, as a body stands on it.
pub fn ground(generator: &Generator, seat: Seat, point: SurfacePoint) -> f64 {
    let direction = seat.sphere(generator).blocks().direction(point);
    ground_m(generator, seat.body(), direction) / BLOCK_M
}

/// Where the volume of a plot starts and ends: the lowest cell it holds and
/// how many it holds above it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Stand {
    pub low: i32,
    pub height: u32,
}

/// Where a volume over the plot a column of a seat is on starts and ends.
/// The ground stays as it is, and the volume holds the blocks from the
/// lowest of it to [`height`] over the highest. A plot on the edge of its
/// sector stays nature, and so does one of the planet whose column is under
/// the sea.
pub fn survey(generator: &Generator, seat: Seat, point: SurfacePoint) -> Result<Stand, Unseated> {
    let sphere = seat.sphere(generator);
    let plot = plot_of(point);
    // The corners of a sector are nature, and a build does not fold over a
    // seam: a plot on the edge of its sector stays as it is.
    let inside = sphere
        .blocks()
        .coarsened(PLOT_BITS)
        .is_some_and(|plots| plot.iter().all(|&n| 0 < n && n < plots.side() as i32 - 1));
    if !inside {
        return Err(Unseated::Seam);
    }
    if seat.body() == Body::Planet && ground(generator, seat, point) < 0.0 {
        return Err(Unseated::Sea);
    }
    let low = plot.map(|n| n << PLOT_BITS);
    let side = 1 << PLOT_BITS;
    let heights = (0..=side).step_by(SURVEY as usize).flat_map(|dv| {
        (0..=side).step_by(SURVEY as usize).map(move |du| {
            let (u, v) = (f64::from(low[0] + du), f64::from(low[1] + dv));
            ground(generator, seat, SurfacePoint::new(point.sector, u, v))
        })
    });
    let (lowest, highest) = heights.fold((f64::MAX, f64::MIN), |(lo, hi), blocks| {
        (lo.min(blocks), hi.max(blocks))
    });
    let bottom = lowest.floor() as i32 - UNDER;
    let top = highest.ceil() as i32 + height(seat.body());
    Ok(Stand {
        low: bottom,
        height: (top - bottom) as u32,
    })
}

/// How far a place is from the volume over a plot of a sector of a body,
/// metres: from the line up the middle of it, foot to top. A body beside a
/// tower is near it, however tall it stands. The place is said from the
/// centre of the body the volume is on, and `sphere` is that body.
pub fn away_m(
    sphere: QuadSphere,
    sector: Sector,
    plot: [i32; 2],
    stand: Stand,
    from: [f64; 3],
) -> f64 {
    let column = middle(sector, plot);
    let foot = sphere.position(column, f64::from(stand.low) * BLOCK_M);
    let top = f64::from(stand.low) + f64::from(stand.height);
    let head = sphere.position(column, top * BLOCK_M);
    let dot = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let (up, to) = (
        [0, 1, 2].map(|i| head[i] - foot[i]),
        [0, 1, 2].map(|i| from[i] - foot[i]),
    );
    // The nearest of the line: where the place falls along it, kept on it.
    let along = (dot(to, up) / dot(up, up).max(f64::MIN_POSITIVE)).clamp(0.0, 1.0);
    let apart = [0, 1, 2].map(|i| to[i] - up[i] * along);
    dot(apart, apart).sqrt()
}

/// A gesture as the wire says it.
pub fn gesture_wire(gesture: Gesture) -> wire::Gesture {
    let (kind, span, paint) = match gesture {
        Gesture::Create { span, paint } => (wire::Kind::Create, span, paint),
        Gesture::Delete { span } => (wire::Kind::Delete, span, 0),
        Gesture::Paint { span, paint } => (wire::Kind::Paint, span, paint),
    };
    wire::Gesture {
        kind: kind.into(),
        x0: span.min[0],
        y0: span.min[1],
        z0: span.min[2],
        x1: span.max[0],
        y1: span.max[1],
        z1: span.max[2],
        paint: u32::from(paint),
    }
}

/// The gesture a message says, when it says one.
pub fn gesture_from_wire(gesture: &wire::Gesture) -> Option<Gesture> {
    let span = Span::between(
        [gesture.x0, gesture.y0, gesture.z0],
        [gesture.x1, gesture.y1, gesture.z1],
    );
    let paint = u8::try_from(gesture.paint).ok()?;
    match gesture.kind() {
        wire::Kind::Create => Some(Gesture::Create { span, paint }),
        wire::Kind::Delete => Some(Gesture::Delete { span }),
        wire::Kind::Paint => Some(Gesture::Paint { span, paint }),
        wire::Kind::Unspecified => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use worldgen::Recipe;

    #[test]
    fn a_gesture_and_a_seat_survive_the_wire() {
        let span = Span::between([-3, 70, 12], [9, 64, -4]);
        for gesture in [
            Gesture::Create { span, paint: 7 },
            Gesture::Delete { span },
            Gesture::Paint { span, paint: 15 },
        ] {
            assert_eq!(gesture_from_wire(&gesture_wire(gesture)), Some(gesture));
        }
        assert_eq!(gesture_from_wire(&wire::Gesture::default()), None);
        let sector = Sector::new(4).unwrap();
        for seat in [Seat::Sector(sector), Seat::Moon(sector)] {
            assert_eq!(Seat::from_wire(Some(&seat.wire())), Some(seat));
            assert_eq!(Seat::from_key(seat.key()), Some(seat));
            assert_eq!(Seat::on(seat.body(), sector), seat);
        }
        assert_eq!(Seat::from_key(6), None);
        assert_eq!(Seat::from_wire(None), None);
        let far = wire::Seat {
            seat: Some(wire::seat::Seat::Sector(99)),
        };
        assert_eq!(Seat::from_wire(Some(&far)), None);
    }

    #[test]
    fn a_volume_holds_the_ground_of_its_plot_and_the_air_over_it() {
        let generator = Generator::new(Recipe::new(1)).unwrap();
        let side = f64::from(generator.sphere().blocks().side());
        let sector = Sector::new(4).unwrap();
        let point = SurfacePoint::new(sector, side * 0.41, side * 0.37);
        let seat = Seat::Sector(sector);
        let stand = survey(&generator, seat, point).expect("dry land");
        let feet = ground(&generator, seat, point) as i32;
        assert!(stand.low < feet);
        assert!(stand.low + stand.height as i32 >= feet + HEIGHT);
        // The same plot, wherever on it the body stands.
        let corner = SurfacePoint::new(sector, point.u + 20.0, point.v - 20.0);
        if plot_of(corner) == plot_of(point) && ground(&generator, seat, corner) >= 0.0 {
            assert_eq!(survey(&generator, seat, corner), Ok(stand));
        }
        assert_eq!(plot_of(middle(sector, plot_of(point))), plot_of(point));
        let edge = SurfacePoint::new(sector, 10.0, point.v);
        assert_eq!(survey(&generator, seat, edge), Err(Unseated::Seam));
    }

    #[test]
    fn a_volume_stands_on_the_moon_by_the_moon_s_own_ground_and_grid() {
        let generator = Generator::new(Recipe::new(1)).unwrap();
        let moon = generator.moon();
        let side = f64::from(moon.blocks().side());
        let seat = Seat::Moon(Sector::new(2).unwrap());
        assert_eq!(seat.sphere(&generator), moon);
        // A crater's floor is under the datum, and no sea fills it.
        let low = (0..64).map(|i| {
            let at = side * (0.2 + 0.01 * f64::from(i));
            SurfacePoint::new(seat.sector(), at, side - at)
        });
        let low = low
            .min_by(|a, b| ground(&generator, seat, *a).total_cmp(&ground(&generator, seat, *b)))
            .unwrap();
        let feet = ground(&generator, seat, low);
        let stand = survey(&generator, seat, low).expect("the moon is all dry");
        assert!(f64::from(stand.low) < feet);
        let top = f64::from(stand.low) + f64::from(stand.height);
        assert!(top >= feet + f64::from(MOON_HEIGHT));
        // Lower than on the planet: 128 metres over its highest ground.
        assert!(top < feet + f64::from(HEIGHT));
        // The edge of a sector of the moon is where its own grid ends.
        let edge = SurfacePoint::new(seat.sector(), side - 10.0, low.v);
        assert_eq!(survey(&generator, seat, edge), Err(Unseated::Seam));
    }
}
