//! Property tests, mandatory for this crate (CLAUDE.md, Tests).

use proptest::prelude::*;
use topology::{Column, Dir, SECTOR_SIDE, Sector, SurfacePoint, vec3};

const MAX: u16 = (SECTOR_SIDE - 1) as u16;

fn sector() -> impl Strategy<Value = Sector> {
    (0u8..6).prop_map(|i| Sector::new(i).unwrap())
}

/// Coordinates biased toward the edges, where seams live.
fn coord() -> impl Strategy<Value = u16> {
    prop_oneof![
        3 => prop_oneof![Just(0), Just(1), Just(MAX - 1), Just(MAX)],
        1 => any::<u16>(),
    ]
}

fn column() -> impl Strategy<Value = Column> {
    (sector(), coord(), coord()).prop_map(|(s, u, v)| Column::new(s, u, v))
}

fn dir() -> impl Strategy<Value = Dir> {
    (0usize..4).prop_map(|i| Dir::ALL[i])
}

proptest! {
    #![proptest_config(ProptestConfig {
        failure_persistence: Some(Box::new(
            proptest::test_runner::FileFailurePersistence::WithSource("regressions"),
        )),
        ..ProptestConfig::default()
    })]

    /// Step, turn around, step: back home, facing the way you came.
    #[test]
    fn step_back_is_identity(c in column(), d in dir()) {
        let there = c.step(d);
        let back = there.column.step(there.dir.opposite());
        prop_assert_eq!(back.column, c);
        prop_assert_eq!(back.dir, d.opposite());
    }

    /// A step moves to an adjacent cell: centres are about one block apart.
    #[test]
    fn step_lands_next_door(c in column(), d in dir()) {
        let here = SurfacePoint::center_of(c).direction();
        let there = SurfacePoint::center_of(c.step(d).column).direction();
        let blocks = vec3::length(vec3::sub(there, here)) * topology::RADIUS_M / topology::BLOCK_M;
        prop_assert!((0.5..1.5).contains(&blocks), "{blocks} blocks");
    }

    /// Four distinct neighbours, none of them self.
    #[test]
    fn neighbours_are_distinct(c in column()) {
        let n = c.neighbours();
        for i in 0..4 {
            prop_assert_ne!(n[i], c);
            for j in 0..i {
                prop_assert_ne!(n[i], n[j]);
            }
        }
    }

    /// direction and from_direction are inverse.
    #[test]
    fn direction_round_trip(s in sector(), u in 0.0..65536.0f64, v in 0.0..65536.0f64) {
        let p = SurfacePoint::new(s, u, v);
        let q = SurfacePoint::from_direction(p.direction());
        prop_assert_eq!(q.sector, s);
        prop_assert!((q.u - u).abs() < 1e-6 && (q.v - v).abs() < 1e-6, "{q:?}");
    }

    /// Folding over a seam is continuous in world space: the overshoot lands
    /// where walking the same distance on the sphere would.
    #[test]
    fn wrap_is_continuous(s in sector(), along in 1.0..65535.0f64, over in 0.0..8.0f64, edge in 0usize..4) {
        let side = f64::from(SECTOR_SIDE);
        let (inside, outside) = match edge {
            0 => ((side, along), (side + over, along)),
            1 => ((0.0, along), (-over, along)),
            2 => ((along, side), (along, side + over)),
            _ => ((along, 0.0), (along, -over)),
        };
        let on_edge = SurfacePoint::new(s, inside.0, inside.1).direction();
        let wrapped = SurfacePoint::new(s, outside.0, outside.1).wrapped();
        prop_assert_ne!(wrapped.sector, s);
        let blocks = vec3::length(vec3::sub(wrapped.direction(), on_edge))
            * topology::RADIUS_M / topology::BLOCK_M;
        // Blocks are near unit size at the seam, never exactly.
        prop_assert!((blocks - over).abs() <= over * 0.5 + 1e-6, "{blocks} vs {over}");
    }

    /// Tangents match finite differences of direction.
    #[test]
    fn tangents_match_finite_differences(s in sector(), u in 1.0..65535.0f64, v in 1.0..65535.0f64) {
        let p = SurfacePoint::new(s, u, v);
        let t = p.tangents();
        let e = 1e-3;
        let fd_u = vec3::scale(vec3::sub(SurfacePoint::new(s, u + e, v).direction(), SurfacePoint::new(s, u - e, v).direction()), 0.5 / e);
        let fd_v = vec3::scale(vec3::sub(SurfacePoint::new(s, u, v + e).direction(), SurfacePoint::new(s, u, v - e).direction()), 0.5 / e);
        prop_assert!(vec3::length(vec3::sub(fd_u, t.du)) < 1e-9 * vec3::length(t.du).max(1.0));
        prop_assert!(vec3::length(vec3::sub(fd_v, t.dv)) < 1e-9 * vec3::length(t.dv).max(1.0));
        prop_assert!(vec3::dot(t.up, t.du).abs() < 1e-12 && vec3::dot(t.up, t.dv).abs() < 1e-12);
    }
}

/// Every seam cell of every sector, exhaustively on a coarse stride, plus the
/// corners: the round trip property with no sampling luck involved.
#[test]
fn every_edge_round_trips() {
    for s in Sector::ALL {
        for k in (0..=MAX).step_by(257).chain([MAX]) {
            for c in [
                Column::new(s, 0, k),
                Column::new(s, MAX, k),
                Column::new(s, k, 0),
                Column::new(s, k, MAX),
            ] {
                for d in Dir::ALL {
                    let there = c.step(d);
                    let back = there.column.step(there.dir.opposite());
                    assert_eq!((back.column, back.dir), (c, d.opposite()), "{c:?} {d:?}");
                }
            }
        }
    }
}

/// Each sector touches four others, never itself, never its opposite.
#[test]
fn each_sector_has_four_neighbours() {
    for s in Sector::ALL {
        let mid = MAX / 2;
        let mut seen: Vec<Sector> = [
            Column::new(s, MAX, mid).step(Dir::UPos),
            Column::new(s, 0, mid).step(Dir::UNeg),
            Column::new(s, mid, MAX).step(Dir::VPos),
            Column::new(s, mid, 0).step(Dir::VNeg),
        ]
        .iter()
        .map(|step| step.column.sector)
        .collect();
        seen.sort();
        seen.dedup();
        assert_eq!(seen.len(), 4);
        assert!(!seen.contains(&s));
    }
}

/// The numbers quoted in project.rs and ARCHITECTURE.md.
#[test]
fn block_distortion_is_bounded() {
    let (mut min, mut max) = (f64::MAX, f64::MIN);
    for i in 0..=64 {
        for j in 0..=64 {
            let p = SurfacePoint::new(Sector::ALL[0], f64::from(i) * 1024.0, f64::from(j) * 1024.0);
            let t = p.tangents();
            for len in [vec3::length(t.du), vec3::length(t.dv)] {
                let metres = len * topology::RADIUS_M;
                min = min.min(metres);
                max = max.max(metres);
            }
        }
    }
    println!(
        "block edge on the sphere: {min:.4} m to {max:.4} m, ratio {:.4}",
        max / min
    );
    assert!(max / min < 1.5);
}
