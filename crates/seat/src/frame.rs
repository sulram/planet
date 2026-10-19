//! The flat frame of a point of a seat, and a part seated in it (DECISIONS
//! 111). A part is rigid in that frame: what is turned in it is turned in
//! the world by the same angle, so a pose folded over a seam onto the next
//! sector stands in the world exactly as it did.

use glam::{DAffine3, DMat3, DQuat, DVec3};
use part::Pose;
use topology::{BLOCK_M, QuadSphere, Sector, SurfacePoint};

/// The frame of a seat at a point of it: the world from flat blocks around
/// the point, counted from the centre of the body. `x` is the sector's `u`
/// made unit, `z` is up and `y` is `z` cross `x`; a block is [`BLOCK_M`]
/// along each. A cube of scale 1 in it is a cell at the middle of a sector,
/// and a little wider than one at its corners, where the grid's blocks
/// shrink.
pub fn frame(sphere: QuadSphere, sector: Sector, at: [f64; 3]) -> DAffine3 {
    let origin =
        DVec3::from(sphere.position(SurfacePoint::new(sector, at[0], at[1]), at[2] * BLOCK_M));
    DAffine3::from_mat3_translation(turned(sphere, sector, at) * BLOCK_M, origin)
}

/// How the frame of a seat at a point of it is turned: its three axes,
/// unit and orthogonal.
fn turned(sphere: QuadSphere, sector: Sector, at: [f64; 3]) -> DMat3 {
    let tangents = sphere
        .blocks()
        .tangents(SurfacePoint::new(sector, at[0], at[1]));
    let up = DVec3::from(tangents.up);
    let x = DVec3::from(tangents.du).normalize();
    DMat3::from_cols(x, up.cross(x), up)
}

/// Where a part in a pose stands in the world, from the centre of its body:
/// the world from the part's own space.
pub fn seated(sphere: QuadSphere, sector: Sector, pose: &Pose) -> DAffine3 {
    frame(sphere, sector, pose.at) * pose.local()
}

/// The seat and pose a part standing in the world has: the sector under its
/// anchor, the address of it there with fractional blocks, and its turn and
/// stretch in the frame of that point. `world` is the world from the part's
/// own space, counted from the centre of its body, as [`seated`] makes it.
/// How a part moves: a hand or a gizmo moves it in the world, and its pose
/// is read back from where it came to, so a part crossing from one sector
/// to the next is the same part at the same place, with nothing folded.
/// `None` for a world that is no pose: not finite, or flat along an axis.
pub fn reseat(sphere: QuadSphere, world: &DAffine3) -> Option<(Sector, Pose)> {
    if !world.is_finite() {
        return None;
    }
    let anchor = world.translation;
    let point = sphere.blocks().surface_point(anchor.to_array());
    let at = [
        point.u,
        point.v,
        (anchor.length() - sphere.radius_m()) / BLOCK_M,
    ];
    // What is left once the frame is taken off is the turn and the stretch.
    let local = turned(sphere, point.sector, at).transpose() * world.matrix3 / BLOCK_M;
    let scale = [local.x_axis, local.y_axis, local.z_axis].map(DVec3::length);
    if scale.iter().any(|&n| n < part::MIN_SCALE) {
        return None;
    }
    let turn = DMat3::from_cols(
        local.x_axis / scale[0],
        local.y_axis / scale[1],
        local.z_axis / scale[2],
    );
    Some((
        point.sector,
        Pose {
            at,
            rotation: DQuat::from_mat3(&turn).normalize(),
            scale,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    const BITS: u32 = 10;

    fn sphere() -> QuadSphere {
        QuadSphere::new(BITS).unwrap()
    }

    #[test]
    fn the_frame_is_orthonormal_blocks_and_up_is_out() {
        let sphere = sphere();
        let half = f64::from(sphere.blocks().side()) / 2.0;
        for sector in Sector::ALL {
            for at in [[half, half, 0.0], [1.0, 1.0, 10.0], [half, 3.0, -4.0]] {
                let frame = frame(sphere, sector, at);
                let [x, y, z] = [frame.x_axis, frame.y_axis, frame.z_axis];
                for axis in [x, y, z] {
                    assert!((axis.length() - BLOCK_M).abs() < 1e-9, "{axis:?}");
                }
                assert!(x.dot(y).abs() < 1e-9 && y.dot(z).abs() < 1e-9 && z.dot(x).abs() < 1e-9);
                assert!(x.cross(y).dot(z) > 0.0, "right handed");
                let out = frame.translation.normalize();
                assert!((z.normalize() - out).length() < 1e-9, "up is out");
                let height_m = frame.translation.length() - sphere.radius_m();
                assert!((height_m - at[2] * BLOCK_M).abs() < 1e-6);
            }
        }
    }

    proptest! {
        /// A part seated anywhere on any sector, read back from where it
        /// stands, is the same pose on the same sector, and stands where it
        /// stood: every corner of its box within a hundredth of a millimetre.
        #[test]
        fn a_part_read_back_from_the_world_is_the_same_part(
            sector in 0u8..6,
            at in prop::array::uniform2(0.001f64..1023.999),
            height in -100.0f64..100.0,
            axis in prop::array::uniform3(-1.0f64..1.0),
            angle in 0.0f64..6.3,
            scale in prop::array::uniform3(0.1f64..8.0),
        ) {
            let sphere = sphere();
            let sector = Sector::new(sector).unwrap();
            let axis = DVec3::from(axis);
            prop_assume!(axis.length() > 1e-3);
            let pose = Pose {
                at: [at[0], at[1], height],
                rotation: DQuat::from_axis_angle(axis.normalize(), angle),
                scale,
            };
            let before = seated(sphere, sector, &pose);
            let (again, read) = reseat(sphere, &before).unwrap();
            prop_assert_eq!(again, sector);
            for (a, b) in read.at.iter().zip(&pose.at) {
                prop_assert!((a - b).abs() < 1e-6, "{:?} vs {:?}", read.at, pose.at);
            }
            for (a, b) in read.scale.iter().zip(&pose.scale) {
                prop_assert!((a - b).abs() < 1e-9, "{:?} vs {:?}", read.scale, pose.scale);
            }
            let after = seated(sphere, again, &read);
            for corner in [[0.0, 0.0, 0.0], [0.5, 0.5, 1.0], [-0.5, 0.5, 0.0], [0.5, -0.5, 1.0]] {
                let corner = DVec3::from(corner);
                let (a, b) = (before.transform_point3(corner), after.transform_point3(corner));
                prop_assert!((a - b).length() < 1e-5, "{a:?} vs {b:?}");
            }
        }

        /// A part pushed in the world over the edge of its sector comes to
        /// the next sector, in range there, and stands where it was pushed
        /// to.
        #[test]
        fn a_part_pushed_over_a_seam_is_on_the_next_sector(
            sector in 0u8..6,
            along in 1.0f64..1023.0,
            edge in 0u8..4,
            push_m in 0.3f64..20.0,
        ) {
            let sphere = sphere();
            let sector = Sector::new(sector).unwrap();
            let side = f64::from(sphere.blocks().side());
            let (at, toward) = match edge {
                0 => ([0.5, along, 0.0], -DVec3::X),
                1 => ([side - 0.5, along, 0.0], DVec3::X),
                2 => ([along, 0.5, 0.0], -DVec3::Y),
                _ => ([along, side - 0.5, 0.0], DVec3::Y),
            };
            let pose = Pose::upright(at, [2.0, 2.0, 2.0]);
            let before = seated(sphere, sector, &pose);
            // Pushed along the frame's own axis, in the world.
            let pushed = DAffine3::from_translation(frame(sphere, sector, at).transform_vector3(toward) / BLOCK_M * push_m) * before;
            let (next, read) = reseat(sphere, &pushed).unwrap();
            prop_assert_ne!(next, sector);
            let inside = |n: f64| (0.0..=side).contains(&n);
            prop_assert!(inside(read.at[0]) && inside(read.at[1]), "{:?}", read.at);
            let after = seated(sphere, next, &read);
            let (a, b) = (pushed.translation, after.translation);
            prop_assert!((a - b).length() < 1e-5, "{a:?} vs {b:?}");
        }
    }
}
