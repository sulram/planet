//! A figure: one body as it is drawn. A VRM avatar moved by locomotion clips,
//! or the box figure while nothing is loaded.
//!
//! One figure per body, the player's and every peer's, over one shared set of
//! clips. What a figure needs to pose itself is a [`Motion`]: the local
//! controller produces one, and a peer's stance interpolates into one, so the
//! figure never knows which body it is.

use std::collections::HashMap;
use std::sync::Arc;

use avatar::{Animator, Avatar, Clip};
use glam::{DAffine3, DMat3, DQuat, DVec3};
use scene::{SkinnedInstance, SkinnedMeshId};

use crate::controller::Controller;
use crate::seam::Mode;

/// How the body moves right now. Each gait is one clip.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Gait {
    Idle,
    Walk,
    Run,
    Jump,
    Fall,
    Fly,
    /// At the surface or under it. Until a swim clip is in the manifest the
    /// fly clip stands in.
    Swim,
}

impl Gait {
    /// The speed the clip was authored for, metres per second. Playback
    /// scales with the real speed: feet do not slide, and a sprint looks like one.
    fn authored_mps(self) -> Option<f64> {
        match self {
            Gait::Walk => Some(1.5),
            Gait::Run => Some(3.4),
            Gait::Swim => Some(2.4),
            _ => None,
        }
    }

    /// On the ground, where legs swing and feet stay put.
    pub fn grounded(self) -> bool {
        matches!(self, Gait::Idle | Gait::Walk | Gait::Run)
    }

    pub fn of(controller: &Controller) -> Gait {
        if controller.mode == Mode::Fly {
            Gait::Fly
        } else if controller.swimming() {
            Gait::Swim
        } else if !controller.grounded() {
            if controller.vertical_mps() > 0.0 {
                Gait::Jump
            } else {
                Gait::Fall
            }
        } else if controller.ground_mps() < 0.1 {
            Gait::Idle
        } else if controller.ground_mps() < 2.2 {
            Gait::Walk
        } else {
            Gait::Run
        }
    }

    /// The wire's name for it.
    pub fn wire(self) -> protocol::Gait {
        match self {
            Gait::Idle => protocol::Gait::Idle,
            Gait::Walk => protocol::Gait::Walk,
            Gait::Run => protocol::Gait::Run,
            Gait::Jump => protocol::Gait::Jump,
            Gait::Fall => protocol::Gait::Fall,
            Gait::Fly => protocol::Gait::Fly,
            Gait::Swim => protocol::Gait::Swim,
        }
    }

    pub fn from_wire(gait: protocol::Gait) -> Gait {
        match gait {
            protocol::Gait::Idle => Gait::Idle,
            protocol::Gait::Walk => Gait::Walk,
            protocol::Gait::Run => Gait::Run,
            protocol::Gait::Jump => Gait::Jump,
            protocol::Gait::Fall => Gait::Fall,
            protocol::Gait::Fly => Gait::Fly,
            protocol::Gait::Swim => Gait::Swim,
        }
    }
}

/// Where a label hangs over a body: just above the head, from the feet.
pub const HEAD_M: f64 = 1.8;

/// What a figure needs to stand and move: where the body is, which way it
/// faces and stands, what it is doing and how fast.
#[derive(Clone, Copy, Debug)]
pub struct Motion {
    /// Feet, world space.
    pub position: DVec3,
    /// The body's frame, columns right, up, back.
    pub basis: DMat3,
    pub gait: Gait,
    /// What the clip plays at: along the ground on foot, through the water
    /// when swimming, through the air in flight.
    pub speed_mps: f64,
    /// Running on foot, flying flat out.
    pub sprint: bool,
    /// Distance walked, for the walk cycle of the box figure.
    pub stride_m: f64,
}

impl Motion {
    pub fn of(controller: &Controller) -> Motion {
        let gait = Gait::of(controller);
        Motion {
            position: controller.position(),
            basis: controller.body_basis(),
            gait,
            speed_mps: speed_for(gait, controller.ground_mps(), controller.speed_mps()),
            sprint: controller.sprinting(),
            stride_m: controller.stride_m(),
        }
    }
}

/// The speed a clip of this gait follows: swimmers and flyers move in three
/// dimensions, so a dive is speed too; on foot only the ground counts.
pub fn speed_for(gait: Gait, ground_mps: f64, speed_mps: f64) -> f64 {
    match gait {
        Gait::Swim | Gait::Fly => speed_mps,
        _ => ground_mps,
    }
}

/// A body's frame from its up and the way it faces: columns right, up, back.
pub fn basis(up: DVec3, facing: DVec3) -> DMat3 {
    let flat = facing - up * facing.dot(up);
    let back = -flat.normalize_or(up.any_orthonormal_vector());
    DMat3::from_cols(up.cross(back), up, back)
}

/// The clips every figure shares, one per gait, retargeted at load.
pub type Clips = HashMap<Gait, Clip>;

const LEAN_CRUISE: f64 = 0.35;
const LEAN_SPRINT: f64 = core::f64::consts::FRAC_PI_4;
const LEAN_SWIM: f64 = 1.25;
const LEAN_PER_S: f64 = 5.0;

/// An avatar on a body: the mesh the renderer holds, and the skeleton that
/// poses it. Shared by every body wearing the same file.
#[derive(Clone)]
pub struct Worn {
    pub mesh: SkinnedMeshId,
    pub avatar: Arc<Avatar>,
}

pub struct Figure {
    worn: Option<Worn>,
    animator: Animator<Gait>,
    /// Forward lean in flight, radians, eased toward its target.
    lean: f64,
}

impl Default for Figure {
    fn default() -> Figure {
        Figure {
            worn: None,
            animator: Animator::new(Gait::Idle),
            lean: 0.0,
        }
    }
}

impl Figure {
    pub fn wear(&mut self, worn: Worn) {
        self.worn = Some(worn);
    }

    /// The mesh worn, if any.
    pub fn mesh(&self) -> Option<SkinnedMeshId> {
        self.worn.as_ref().map(|worn| worn.mesh)
    }

    pub fn update(&mut self, dt: f64, motion: &Motion, clips: &Clips) {
        let gait = motion.gait;
        let speed = gait.authored_mps().map_or(1.0, |authored| {
            (motion.speed_mps / authored).clamp(0.6, 2.2)
        });
        self.animator.update(dt as f32, gait, speed as f32);

        // Superman leans into the flight: a little when cruising, 45 degrees
        // at a sprint, upright when hovering. Eased, so Shift reads as a dive
        // into speed and not a snap.
        let moving = motion.speed_mps > 0.1;
        let target = match (gait, moving, motion.sprint) {
            (Gait::Fly, true, true) => LEAN_SPRINT,
            (Gait::Fly, true, false) => LEAN_CRUISE,
            // A swim clip lies in the water by itself. The fly clip standing
            // in for it is upright, so the body leans instead.
            (Gait::Swim, true, _) if !clips.contains_key(&Gait::Swim) => LEAN_SWIM,
            _ => 0.0,
        };
        self.lean += (target - self.lean) * (1.0 - (-LEAN_PER_S * dt).exp());
    }

    /// The posed avatar, when one is worn.
    pub fn instance(&self, motion: &Motion, clips: &Clips) -> Option<SkinnedInstance> {
        let worn = self.worn.as_ref()?;
        // VRM 0.x looks down its `-Z`, the same way the body frame does.
        let mut transform = DAffine3::from_mat3_translation(motion.basis, motion.position);
        if self.lean > 1e-4 {
            let hips = DVec3::new(0.0, 0.9, 0.0);
            transform = transform
                * DAffine3::from_translation(hips)
                * DAffine3::from_quat(DQuat::from_rotation_x(-self.lean))
                * DAffine3::from_translation(-hips);
        }
        let pose = self.animator.pose(|gait| {
            let stand_in = (gait == Gait::Swim)
                .then(|| clips.get(&Gait::Fly))
                .flatten();
            clips.get(&gait).or(stand_in)
        });
        // Clips arrive after the avatar over a network: until then, the rest pose.
        let joints = worn
            .avatar
            .joint_matrices(&pose.unwrap_or_else(avatar::Pose::rest));
        Some(SkinnedInstance {
            mesh: worn.mesh,
            transform,
            joints,
        })
    }
}
