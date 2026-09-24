//! The other bodies in the world: who they are, where they were last seen,
//! and where to draw them now.
//!
//! A stance arrives a few times a second and a frame is drawn sixty times a
//! second, so a peer is drawn a little in the past, between the last two
//! stances heard, and holds at the newest one when the next is late. The
//! stance is an address; it becomes a world position here, once, when it
//! lands, so a walk across a sector seam interpolates in world space and
//! never through the seam.

use std::collections::HashMap;

use glam::DVec3;
use topology::{Sector, SurfacePoint};
use worldgen::{Generator, MOON_RADIUS_M};

use crate::figure::{self, Clips, Figure, Gait, Motion};
use crate::seam::PeerInfo;

/// How far behind the newest stance a peer is drawn: a tick and a half, so
/// there is nearly always a next stance to move toward.
const DELAY_S: f64 = 1.5 / crate::session::STANCE_HZ;

/// Where a stance put a body, relative to the body it stands on.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Anchor {
    /// From the planet's centre.
    Planet(DVec3),
    /// From the moon's centre, wherever the moon is by then.
    Moon(DVec3),
}

impl Anchor {
    fn position(self, moon: DVec3) -> DVec3 {
        match self {
            Anchor::Planet(offset) => offset,
            Anchor::Moon(offset) => moon + offset,
        }
    }

    fn up(self) -> DVec3 {
        match self {
            Anchor::Planet(offset) | Anchor::Moon(offset) => offset.normalize_or(DVec3::Y),
        }
    }

    fn same_body(self, other: Anchor) -> bool {
        matches!(
            (self, other),
            (Anchor::Planet(_), Anchor::Planet(_)) | (Anchor::Moon(_), Anchor::Moon(_))
        )
    }
}

/// One stance, as heard.
#[derive(Clone, Copy, Debug)]
struct Sample {
    at_s: f64,
    anchor: Anchor,
    facing: DVec3,
    gait: Gait,
    speed_mps: f64,
    sprint: bool,
}

impl Sample {
    fn read(stance: &protocol::Stance, generator: &Generator, at_s: f64) -> Option<Sample> {
        let sector = Sector::new(u8::try_from(stance.sector).ok()?)?;
        let sphere = generator.sphere();
        let point = SurfacePoint::new(sector, f64::from(stance.u), f64::from(stance.v));
        let direction = DVec3::from(sphere.blocks().direction(point));
        let height_m = f64::from(stance.height_m);
        let anchor = match stance.body() {
            protocol::Body::Planet => Anchor::Planet(direction * (sphere.radius_m() + height_m)),
            protocol::Body::Moon => Anchor::Moon(direction * (MOON_RADIUS_M + height_m)),
        };
        let facing = DVec3::new(
            f64::from(stance.facing_x),
            f64::from(stance.facing_y),
            f64::from(stance.facing_z),
        );
        Some(Sample {
            at_s,
            anchor,
            facing: facing.normalize_or(anchor.up().any_orthonormal_vector()),
            gait: Gait::from_wire(stance.gait()),
            speed_mps: f64::from(stance.speed_mps),
            sprint: stance.sprint,
        })
    }
}

pub struct Peer {
    pub info: PeerInfo,
    /// The asset reference of what this peer wears.
    pub avatar: String,
    pub figure: Figure,
    /// The two newest stances: where the body is drawn between.
    previous: Option<Sample>,
    latest: Option<Sample>,
    stride_m: f64,
}

impl Peer {
    fn new(info: PeerInfo, avatar: String) -> Peer {
        Peer {
            info,
            avatar,
            figure: Figure::default(),
            previous: None,
            latest: None,
            stride_m: 0.0,
        }
    }

    fn heard(&mut self, sample: Sample) {
        self.previous = self.latest.take();
        self.latest = Some(sample);
    }

    /// Where to draw the body at `now`, or nowhere until a stance has come.
    fn motion_at(&self, now_s: f64, moon: DVec3) -> Option<Motion> {
        let latest = self.latest?;
        let render_s = now_s - DELAY_S;
        let (anchor, facing) = match self.previous {
            Some(previous)
                if previous.anchor.same_body(latest.anchor) && render_s < latest.at_s =>
            {
                let span = latest.at_s - previous.at_s;
                let t = if span > 1e-6 {
                    ((render_s - previous.at_s) / span).clamp(0.0, 1.0)
                } else {
                    1.0
                };
                let anchor = match (previous.anchor, latest.anchor) {
                    (Anchor::Planet(a), Anchor::Planet(b)) => Anchor::Planet(a.lerp(b, t)),
                    (Anchor::Moon(a), Anchor::Moon(b)) => Anchor::Moon(a.lerp(b, t)),
                    (_, b) => b,
                };
                let facing = previous
                    .facing
                    .lerp(latest.facing, t)
                    .normalize_or(latest.facing);
                (anchor, facing)
            }
            _ => (latest.anchor, latest.facing),
        };
        Some(Motion {
            position: anchor.position(moon),
            basis: figure::basis(anchor.up(), facing),
            gait: latest.gait,
            speed_mps: latest.speed_mps,
            sprint: latest.sprint,
            stride_m: self.stride_m,
        })
    }
}

#[derive(Default)]
pub struct Peers {
    by_session: HashMap<u32, Peer>,
    /// The client's clock, seconds: what samples are stamped with.
    now_s: f64,
}

impl Peers {
    pub fn add(&mut self, peer: &protocol::Peer, generator: &Generator) {
        let info = PeerInfo {
            session: peer.session,
            name: peer.name.clone(),
            visitor: peer.visitor,
        };
        let mut added = Peer::new(info, peer.avatar.clone());
        if let Some(sample) = peer
            .stance
            .as_ref()
            .and_then(|stance| Sample::read(stance, generator, self.now_s))
        {
            added.heard(sample);
        }
        self.by_session.insert(peer.session, added);
    }

    pub fn remove(&mut self, session: u32) -> Option<Peer> {
        self.by_session.remove(&session)
    }

    pub fn clear(&mut self) {
        self.by_session.clear();
    }

    pub fn moved(&mut self, session: u32, stance: &protocol::Stance, generator: &Generator) {
        if let (Some(peer), Some(sample)) = (
            self.by_session.get_mut(&session),
            Sample::read(stance, generator, self.now_s),
        ) {
            peer.heard(sample);
        }
    }

    pub fn wearing(&mut self, session: u32, avatar: &str) {
        if let Some(peer) = self.by_session.get_mut(&session) {
            peer.avatar = avatar.to_owned();
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &Peer> {
        self.by_session.values()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Peer> {
        self.by_session.values_mut()
    }

    /// Who is here, for the seam. In session order, so a list stays put.
    pub fn infos(&self) -> Vec<PeerInfo> {
        let mut infos: Vec<PeerInfo> = self.by_session.values().map(|p| p.info.clone()).collect();
        infos.sort_by_key(|info| info.session);
        infos
    }

    /// Time passes: every peer's figure moves on.
    pub fn update(&mut self, dt: f64, moon: DVec3, clips: &Clips) {
        self.now_s += dt;
        for peer in self.by_session.values_mut() {
            let Some(motion) = peer.motion_at(self.now_s, moon) else {
                continue;
            };
            if motion.gait.grounded() {
                peer.stride_m += motion.speed_mps * dt;
            }
            peer.figure.update(dt, &motion, clips);
        }
    }

    /// Every peer that has said where it is, and where to draw it now.
    /// Where each peer's head is now, world space, by session: what a label
    /// is hung over.
    pub fn heads(&self, moon: DVec3) -> impl Iterator<Item = (u32, DVec3)> {
        self.by_session.iter().filter_map(move |(session, peer)| {
            let motion = peer.motion_at(self.now_s, moon)?;
            Some((
                *session,
                motion.position + motion.basis.y_axis * figure::HEAD_M,
            ))
        })
    }

    pub fn bodies(&self, moon: DVec3) -> impl Iterator<Item = (&Figure, Motion)> {
        self.by_session
            .values()
            .filter_map(move |peer| Some((&peer.figure, peer.motion_at(self.now_s, moon)?)))
    }
}
