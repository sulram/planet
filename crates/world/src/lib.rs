//! What a plugin's world half stands on (DECISIONS 93, 99): the plugin as the
//! server hosts it, and the room it is handed while it applies an op. And
//! the systems of the core that run where the world is decided, the cells
//! ([`cells`]), which are owners as a plugin is and stand on the same room.
//!
//! A world half is the part of a plugin that runs where the world is decided.
//! It is the mirror of the client half's `client::Plugin` and its host: the
//! server's module holds each world half and hands it a [`Room`], and a test
//! hands it one of its own. A world half imports this crate and never
//! `client`.

use core::time::Duration;

use protocol::Stance;
use topology::{BLOCK_M, QuadSphere, Sector, SurfacePoint, vec3};
use worldgen::Generator;

pub mod cells;
mod host;

pub use host::{GroundOf, Host, Installed, MeasureOf};
pub use protocol::Level;

/// One thing a session may ask of a plugin, and the least level that may ask
/// it. What a plugin offers is said as data, so a front end and an agent read
/// the same list (DECISIONS 94).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Op {
    pub kind: &'static str,
    pub level: Level,
}

/// A session as a plugin sees it: one of the core's three words, beside what
/// and where.
#[derive(Clone, PartialEq, Debug)]
pub struct Who {
    /// Given by the world's actor, unique while the world is active.
    pub session: u32,
    /// The account's id in mundos. Empty for a visitor.
    pub user: String,
    /// What the person is called here.
    pub name: String,
    /// What the session may do, for its life.
    pub level: Level,
    /// Where the body stands, as the actor holds it. `None` until it has said.
    pub stance: Option<Stance>,
}

/// A moment on the server's clock: milliseconds since the Unix epoch.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Debug)]
pub struct Moment(pub u64);

impl Moment {
    /// How long passed from `earlier` to this moment. Nothing, when `earlier`
    /// is the later of the two.
    pub fn since(self, earlier: Moment) -> Duration {
        Duration::from_millis(self.0.saturating_sub(earlier.0))
    }
}

/// The size of the bodies of a world, as its recipe says them: what turns two
/// stances into a distance.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Measure {
    /// The planet, and the grid every stance is said on: the moon's too.
    pub sphere: QuadSphere,
    /// The moon: its size, and the grid what is built on it is cut by.
    pub moon: QuadSphere,
}

impl Measure {
    /// A body of this world: its size, and the grid its cells are cut by.
    pub fn body(&self, body: protocol::Body) -> QuadSphere {
        match body {
            protocol::Body::Moon => self.moon,
            _ => self.sphere,
        }
    }

    /// Where a stance is, metres from the centre of the body it is on.
    /// `None` for a stance that names no sector.
    pub fn position(&self, stance: &Stance) -> Option<[f64; 3]> {
        let sector = Sector::new(u8::try_from(stance.sector).ok()?)?;
        let point = SurfacePoint::new(sector, f64::from(stance.u), f64::from(stance.v));
        let height_m = f64::from(stance.height_m);
        match stance.body() {
            protocol::Body::Planet => Some(self.sphere.position(point, height_m)),
            protocol::Body::Moon => {
                let from_centre = self.moon.radius_m() + height_m;
                let direction = self.sphere.blocks().direction(point);
                Some(direction.map(|axis| axis * from_centre))
            }
        }
    }

    /// How far apart two stances stand, in blocks: the great circle along the
    /// datum and the difference in height, taken together. `None` when they
    /// are on different bodies, or one names a sector that does not exist.
    pub fn apart(&self, a: &Stance, b: &Stance) -> Option<f64> {
        if a.body != b.body {
            return None;
        }
        let dot = vec3::dot(self.direction(a)?, self.direction(b)?);
        let along = libm::acos(dot.clamp(-1.0, 1.0)) * self.radius_blocks(a.body());
        let up = (f64::from(a.height_m) - f64::from(b.height_m)) / BLOCK_M;
        Some(libm::hypot(along, up))
    }

    /// The unit vector from the body's centre through where a stance stands.
    fn direction(&self, stance: &Stance) -> Option<topology::Vec3> {
        let sector = Sector::new(u8::try_from(stance.sector).ok()?)?;
        let point = SurfacePoint::new(sector, f64::from(stance.u), f64::from(stance.v));
        Some(self.sphere.blocks().direction(point))
    }

    /// The datum radius of a body, in blocks.
    fn radius_blocks(&self, body: protocol::Body) -> f64 {
        match body {
            protocol::Body::Planet => self.sphere.radius_blocks(),
            protocol::Body::Moon => self.moon.radius_blocks(),
        }
    }
}

/// What a world half is handed while it applies an op: the world as the
/// server holds it at that moment, a way to tell an event to the sessions it
/// picks, and the owner's store. A test hands it one of its own.
pub trait Room {
    /// The server's moment for this op.
    fn now(&self) -> Moment;

    /// How this world's bodies measure.
    fn measure(&self) -> Measure;

    /// Everyone in the world, in the order of their sessions.
    fn sessions(&self) -> &[Who];

    /// Says an event of this owner to the sessions picked.
    fn tell(&mut self, kind: &str, payload: Vec<u8>, to: &[u32]);

    /// The ground of this world, to read. `None` in a world whose ground
    /// the server does not hold: one shaped by a field (OPEN).
    fn ground(&self) -> Option<&Generator>;

    /// What this owner keeps under a key, in its store in the world folder.
    fn get(&mut self, key: &[u8]) -> Option<Vec<u8>>;

    /// Every row this owner keeps whose key starts with a prefix, in the
    /// order of the keys.
    fn scan(&mut self, prefix: &[u8]) -> Vec<(Vec<u8>, Vec<u8>)>;

    /// Keeps a value under a key. What an op keeps is written when the op
    /// has been applied, all of it or none.
    fn keep(&mut self, key: &[u8], value: Vec<u8>);

    /// Forgets a key.
    fn forget(&mut self, key: &[u8]);

    /// Answers the op being applied with the code of why it is refused. An
    /// op nothing refuses has landed.
    fn refuse(&mut self, code: &str);
}

/// A plugin's world half. The host calls it one call at a time, and may keep
/// it behind a lock or hand it to the task that runs a world: it is `Send`.
pub trait Plugin: Send {
    /// The name the wire's envelope and the world's statement say.
    fn name(&self) -> &'static str;

    /// The version of its wire and of its seam.
    fn version(&self) -> u32;

    /// What a session may ask of it.
    fn ops(&self) -> Vec<Op>;

    /// Applies an op the host let through: the plugin is on, the kind is one
    /// of its ops and the session's level may ask it.
    fn op(&mut self, kind: &str, payload: &[u8], who: &Who, room: &mut dyn Room);

    /// A session left, so what was kept for it is dropped.
    fn gone(&mut self, session: u32);
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol::Body;

    /// The moon of every generator so far, `worldgen::MOON_BITS`.
    const MOON_BITS: u32 = 14;

    fn measure(bits: u32) -> Measure {
        Measure {
            sphere: QuadSphere::new(bits).unwrap(),
            moon: QuadSphere::new(MOON_BITS).unwrap(),
        }
    }

    fn at(body: Body, sector: u32, u: f32, v: f32, height_m: f32) -> Stance {
        Stance {
            body: body.into(),
            sector,
            u,
            v,
            height_m,
            ..Stance::default()
        }
    }

    #[test]
    fn apart_is_blocks() {
        let m = measure(16);
        let centre = at(Body::Planet, 2, 32768.0, 32768.0, 0.0);

        // A block is a block at a sector centre, along the ground and up.
        let along = m.apart(&centre, &at(Body::Planet, 2, 32778.0, 32768.0, 0.0));
        assert!(
            (along.unwrap() - 10.0).abs() < 0.01,
            "ten blocks along u: {along:?}"
        );
        let up = m.apart(&centre, &at(Body::Planet, 2, 32768.0, 32768.0, 5.0));
        assert!(
            (up.unwrap() - 10.0).abs() < 1e-9,
            "five metres up is ten blocks: {up:?}"
        );

        // Across a seam the distance is still the distance. Sector 0 looks out
        // along +X with v along +Z; sector 4 looks out along +Z with u along
        // +X. A block short of the seam on each side.
        let edge = at(Body::Planet, 0, 32768.0, 65535.0, 0.0);
        let over = at(Body::Planet, 4, 65535.0, 32768.0, 0.0);
        assert!(
            m.apart(&edge, &over).unwrap() < 3.0,
            "a step over the seam is a step"
        );
    }

    #[test]
    fn the_moon_is_its_own_body_and_smaller() {
        let m = measure(16);
        let planet = at(Body::Planet, 2, 32768.0, 32768.0, 0.0);
        let moon = at(Body::Moon, 2, 32768.0, 32768.0, 0.0);
        assert_eq!(
            m.apart(&planet, &moon),
            None,
            "the moon is not near the planet"
        );
        let step = m.apart(&moon, &at(Body::Moon, 2, 32778.0, 32768.0, 0.0));
        assert!(
            step.unwrap() < 5.0,
            "ten planet blocks are fewer on the moon: {step:?}"
        );
    }

    #[test]
    fn nowhere_is_not_near() {
        let m = measure(16);
        let centre = at(Body::Planet, 2, 32768.0, 32768.0, 0.0);
        assert_eq!(m.apart(&centre, &at(Body::Planet, 6, 0.0, 0.0, 0.0)), None);
    }

    /// A recipe names its own size (DECISIONS 49): ten blocks are ten blocks on
    /// a small world too, where a measure fixed at `2^16` reads them as fewer.
    #[test]
    fn a_small_world_is_measured_by_its_own_size() {
        for bits in [8, 12, 16] {
            let m = measure(bits);
            let half = (1u32 << (bits - 1)) as f32;
            let centre = at(Body::Planet, 2, half, half, 0.0);
            let along = m.apart(&centre, &at(Body::Planet, 2, half + 10.0, half, 0.0));
            assert!(
                (along.unwrap() - 10.0).abs() < 0.05,
                "ten blocks along u at 2^{bits}: {along:?}"
            );
        }
    }

    #[test]
    fn a_moment_counts_forward() {
        assert_eq!(Moment(5_000).since(Moment(2_000)), Duration::from_secs(3));
        assert_eq!(Moment(2_000).since(Moment(5_000)), Duration::ZERO);
    }
}
