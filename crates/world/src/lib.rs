//! What a plugin's world half stands on (DECISIONS 93, 99): the plugin as the
//! server hosts it, and the room it is handed while it applies an op.
//!
//! A world half is the part of a plugin that runs where the world is decided.
//! It is the mirror of the client half's `client::Plugin` and its host: the
//! server's module holds each world half and hands it a [`Room`], and a test
//! hands it one of its own. A world half imports this crate and never
//! `client`.

use core::time::Duration;

use protocol::Stance;
use topology::{BLOCK_M, QuadSphere, Sector, SurfacePoint, vec3};

mod host;

pub use host::{Host, Installed, MeasureOf};
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
    /// The moon's datum radius.
    pub moon_radius_m: f64,
}

impl Measure {
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
            protocol::Body::Moon => self.moon_radius_m / BLOCK_M,
        }
    }
}

/// What the host offers a world half while it applies an op. It carries no
/// feature: a plugin picks whom an event reaches and measures its own reach.
pub trait Room {
    /// The moment the op is applied.
    fn now(&self) -> Moment;

    /// The size of this world's bodies.
    fn measure(&self) -> Measure;

    /// Everyone in the world, the session that asked included.
    fn sessions(&self) -> &[Who];

    /// Says one event of the plugin, encoded once, to the sessions named.
    fn tell(&mut self, kind: &str, payload: Vec<u8>, to: &[u32]);
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

    /// The moon of every generator so far, `worldgen::MOON_RADIUS_M`.
    const MOON_RADIUS_M: f64 = 8000.0;

    fn measure(bits: u32) -> Measure {
        Measure {
            sphere: QuadSphere::new(bits).unwrap(),
            moon_radius_m: MOON_RADIUS_M,
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
