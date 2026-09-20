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
}

impl Gait {
    /// The ground speed the clip was authored for, metres per second.
    /// Playback scales with the real speed so feet do not slide much.
    fn authored_mps(self) -> Option<f64> {
        match self {
            Gait::Walk => Some(1.5),
            Gait::Run => Some(3.4),
            _ => None,
        }
    }

    fn of(controller: &Controller) -> Gait {
        if controller.mode == Mode::Fly {
            Gait::Fly
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

pub struct Body {
    clips: HashMap<Gait, Clip>,
    avatar: Option<(SkinnedMeshId, Avatar)>,
    animator: Animator<Gait>,
    next_mesh: u64,
    changes: Vec<SkinnedChange>,
}

impl Default for Body {
    fn default() -> Body {
        Body {
            clips: HashMap::new(),
            avatar: None,
            animator: Animator::new(Gait::Idle),
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
            (controller.ground_mps() / authored).clamp(0.6, 2.2)
        });
        self.animator.update(dt as f32, gait, speed as f32);
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
        if controller.mode == Mode::Fly {
            // Superman: lean into the flight, more with speed.
            let lean = -1.2 * (controller.speed_mps() / 40.0).min(1.0);
            let hips = glam::DVec3::new(0.0, 0.9, 0.0);
            transform = transform
                * DAffine3::from_translation(hips)
                * DAffine3::from_quat(DQuat::from_rotation_x(lean))
                * DAffine3::from_translation(-hips);
        }
        let pose = self.animator.pose(|gait| self.clips.get(&gait));
        // Clips arrive after the avatar over a network: until then, the rest pose.
        let joints = avatar.joint_matrices(&pose.unwrap_or_else(avatar::Pose::rest));
        Some(SkinnedInstance {
            mesh: *id,
            transform,
            joints,
        })
    }
}
