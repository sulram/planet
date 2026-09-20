//! The body the player wears: a VRM avatar moved by locomotion clips, or the
//! box figure while nothing is loaded.

use std::collections::HashMap;

use avatar::{Animator, Avatar, Clip};
use glam::{DAffine3, DMat3, DQuat};
use scene::{SkinnedChange, SkinnedInstance, SkinnedMeshId};

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

    fn of(controller: &Controller) -> Gait {
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
}

const LEAN_CRUISE: f64 = 0.35;
const LEAN_SPRINT: f64 = core::f64::consts::FRAC_PI_4;
const LEAN_SWIM: f64 = 1.25;
const LEAN_PER_S: f64 = 5.0;

pub struct Body {
    clips: HashMap<Gait, Clip>,
    avatar: Option<(SkinnedMeshId, Avatar)>,
    animator: Animator<Gait>,
    /// Forward lean in flight, radians, eased toward its target.
    lean: f64,
    next_mesh: u64,
    changes: Vec<SkinnedChange>,
}

impl Default for Body {
    fn default() -> Body {
        Body {
            clips: HashMap::new(),
            avatar: None,
            animator: Animator::new(Gait::Idle),
            lean: 0.0,
            next_mesh: 0,
            changes: Vec::new(),
        }
    }
}

impl Body {
    pub fn add_clip(&mut self, gait: Gait, clip: Clip) {
        self.clips.insert(gait, clip);
    }

    /// Wears a new avatar, dropping the old one.
    pub fn wear(&mut self, avatar: Avatar) {
        if let Some((old, _)) = self.avatar.take() {
            self.changes.push(SkinnedChange::Remove(old));
        }
        self.next_mesh += 1;
        let id = SkinnedMeshId(self.next_mesh);
        self.changes
            .push(SkinnedChange::Add(id, avatar.mesh.clone()));
        self.avatar = Some((id, avatar));
    }

    pub fn is_worn(&self) -> bool {
        self.avatar.is_some()
    }

    pub fn drain_changes(&mut self) -> Vec<SkinnedChange> {
        core::mem::take(&mut self.changes)
    }

    pub fn update(&mut self, dt: f64, controller: &Controller) {
        let gait = Gait::of(controller);
        let speed = gait.authored_mps().map_or(1.0, |authored| {
            // Swimmers move in three dimensions: a dive is speed too.
            let real = if gait == Gait::Swim {
                controller.speed_mps()
            } else {
                controller.ground_mps()
            };
            (real / authored).clamp(0.6, 2.2)
        });
        self.animator.update(dt as f32, gait, speed as f32);

        // Superman leans into the flight: a little when cruising, 45 degrees
        // at a sprint, upright when hovering. Eased, so Shift reads as a dive
        // into speed and not a snap.
        let target = match (
            controller.mode,
            controller.speed_mps() > 0.1,
            controller.sprinting(),
        ) {
            (Mode::Fly, true, true) => LEAN_SPRINT,
            (Mode::Fly, true, false) => LEAN_CRUISE,
            // A swim clip lies in the water by itself. The fly clip standing
            // in for it is upright, so the body leans instead.
            (Mode::Walk, true, _)
                if controller.swimming() && !self.clips.contains_key(&Gait::Swim) =>
            {
                LEAN_SWIM
            }
            _ => 0.0,
        };
        self.lean += (target - self.lean) * (1.0 - (-LEAN_PER_S * dt).exp());
    }

    /// The posed avatar, when one is worn.
    pub fn instance(&self, controller: &Controller) -> Option<SkinnedInstance> {
        let (id, avatar) = self.avatar.as_ref()?;
        let up = controller.up();
        let back = -controller.facing();
        // VRM 0.x looks down its `-Z`, the same way the body frame does.
        let mut transform = DAffine3::from_mat3_translation(
            DMat3::from_cols(up.cross(back), up, back),
            controller.position(),
        );
        if self.lean > 1e-4 {
            let hips = glam::DVec3::new(0.0, 0.9, 0.0);
            transform = transform
                * DAffine3::from_translation(hips)
                * DAffine3::from_quat(DQuat::from_rotation_x(-self.lean))
                * DAffine3::from_translation(-hips);
        }
        let pose = self.animator.pose(|gait| {
            let stand_in = (gait == Gait::Swim)
                .then(|| self.clips.get(&Gait::Fly))
                .flatten();
            self.clips.get(&gait).or(stand_in)
        });
        // Clips arrive after the avatar over a network: until then, the rest pose.
        let joints = avatar.joint_matrices(&pose.unwrap_or_else(avatar::Pose::rest));
        Some(SkinnedInstance {
            mesh: *id,
            transform,
            joints,
        })
    }
}
