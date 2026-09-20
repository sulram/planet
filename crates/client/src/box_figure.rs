//! The fallback body: a figure of boxes with a walk cycle.
//!
//! Shown until a VRM avatar is loaded, and when none can be (no assets, a bad
//! file). Procedural, so it is always there.

use glam::{DAffine3, DMat3, DQuat, DVec3, Vec3};
use scene::BoxPart;

use crate::controller::Controller;
use crate::seam::Mode;

const SKIN: Vec3 = Vec3::new(0.92, 0.92, 0.92);
const CLOTH: Vec3 = Vec3::new(0.03, 0.03, 0.03);

/// Metres of walking per full swing of the legs.
const STRIDE_M: f64 = 1.6;

/// A limb or body part in avatar space: `x` right, `y` up, `z` back.
struct Part {
    /// The joint the part swings around.
    pivot: DVec3,
    /// Centre of the box relative to the pivot.
    offset: DVec3,
    size: DVec3,
    color: Vec3,
    /// Swing around `x`, as a factor of the cycle: legs and arms alternate.
    swing: f64,
}

const PARTS: [Part; 6] = [
    Part {
        pivot: DVec3::new(0.0, 1.10, 0.0),
        offset: DVec3::new(0.0, 0.0, 0.0),
        size: DVec3::new(0.44, 0.60, 0.24),
        color: CLOTH,
        swing: 0.0,
    },
    Part {
        pivot: DVec3::new(0.0, 1.42, 0.0),
        offset: DVec3::new(0.0, 0.16, 0.0),
        size: DVec3::new(0.26, 0.28, 0.26),
        color: SKIN,
        swing: 0.0,
    },
    Part {
        pivot: DVec3::new(-0.12, 0.80, 0.0),
        offset: DVec3::new(0.0, -0.40, 0.0),
        size: DVec3::new(0.18, 0.80, 0.20),
        color: CLOTH,
        swing: 1.0,
    },
    Part {
        pivot: DVec3::new(0.12, 0.80, 0.0),
        offset: DVec3::new(0.0, -0.40, 0.0),
        size: DVec3::new(0.18, 0.80, 0.20),
        color: CLOTH,
        swing: -1.0,
    },
    Part {
        pivot: DVec3::new(-0.30, 1.38, 0.0),
        offset: DVec3::new(0.0, -0.30, 0.0),
        size: DVec3::new(0.14, 0.64, 0.16),
        color: SKIN,
        swing: -1.0,
    },
    Part {
        pivot: DVec3::new(0.30, 1.38, 0.0),
        offset: DVec3::new(0.0, -0.30, 0.0),
        size: DVec3::new(0.14, 0.64, 0.16),
        color: SKIN,
        swing: 1.0,
    },
];

/// The avatar's boxes in world space for the controller's current pose.
pub fn parts(controller: &Controller) -> Vec<BoxPart> {
    let up = controller.up();
    let back = -controller.facing();
    let body = DAffine3::from_mat3_translation(
        DMat3::from_cols(up.cross(back), up, back),
        controller.position(),
    );

    let flying = controller.mode == Mode::Fly;
    // Superman: the whole figure tips forward around the hips, arms ahead.
    let speed = (controller.speed_mps() / 40.0).min(1.0);
    let lean = if flying {
        DQuat::from_rotation_x(-1.35 * speed)
    } else {
        DQuat::IDENTITY
    };
    let hips = DVec3::new(0.0, 0.80, 0.0);
    let pose = DAffine3::from_translation(hips)
        * DAffine3::from_quat(lean)
        * DAffine3::from_translation(-hips);

    let cycle = (controller.stride_m() / STRIDE_M * core::f64::consts::TAU).sin();
    let moving = (controller.speed_mps() / 5.0).min(1.0);
    PARTS
        .iter()
        .map(|part| {
            let is_arm = part.pivot.y > 1.3 && part.swing != 0.0;
            let angle = if flying {
                if is_arm { -2.9 * speed } else { 0.0 }
            } else if controller.grounded() {
                part.swing * cycle * 0.7 * moving
            } else {
                part.swing * 0.5
            };
            let local = DAffine3::from_translation(part.pivot)
                * DAffine3::from_quat(DQuat::from_rotation_x(angle))
                * DAffine3::from_translation(part.offset)
                * DAffine3::from_scale(part.size);
            BoxPart {
                transform: body * pose * local,
                color: part.color,
            }
        })
        .collect()
}
