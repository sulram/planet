//! Property tests, mandatory for this crate (CLAUDE.md, Tests).
//!
//! Every property is asked of a body of any legal size, not only of today's
//! planet: a seam that resolves at `2^16` and not at `2^4` is a seam that does
//! not resolve.

use proptest::prelude::*;
use topology::{Column, Dir, Grid, MAX_BITS, MIN_BITS, QuadSphere, Sector, SurfacePoint, vec3};

fn sector() -> impl Strategy<Value = Sector> {
    (0u8..6).prop_map(|i| Sector::new(i).unwrap())
}

/// Any grid, of any grain: a block grid, a chunk grid, a reduced level.
fn grid() -> impl Strategy<Value = Grid> {
    (0u32..=MAX_BITS).prop_map(|bits| Grid::new(bits).unwrap())
}

/// A column of a given grid, biased toward the edges, where seams live.
fn column_of(grid: Grid) -> impl Strategy<Value = Column> {
    let max = grid.max_coord();
    let coord = prop_oneof![
        3 => prop_oneof![Just(0u16), Just(1.min(max)), Just(max.saturating_sub(1)), Just(max)],
        1 => 0..=max,
    ];
    (sector(), coord.clone(), coord).prop_map(|(s, u, v)| Column::new(s, u, v))
}

/// A grid and a cell on it.
fn cell() -> impl Strategy<Value = (Grid, Column)> {
    grid().prop_flat_map(|g| (Just(g), column_of(g)))
}

/// A grid and a point on it, in cell units.
fn point() -> impl Strategy<Value = (Grid, SurfacePoint)> {
    grid().prop_flat_map(|g| {
        let span = f64::from(g.side());
        (Just(g), sector(), 0.0..span, 0.0..span)
            .prop_map(|(g, sec, u, v)| (g, SurfacePoint::new(sec, u, v)))
    })
}

fn dir() -> impl Strategy<Value = Dir> {
    (0usize..4).prop_map(|i| Dir::ALL[i])
}

/// Distance between two unit directions, in cells of a grid. A grid has no
/// metres, so the yardstick is its own circumference: four sides to a great
/// circle, as on any body.
fn cells_apart(grid: Grid, a: [f64; 3], b: [f64; 3]) -> f64 {
    vec3::length(vec3::sub(b, a)) * f64::from(grid.side()) * 4.0 / core::f64::consts::TAU
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
    fn step_back_is_identity((grid, c) in cell(), d in dir()) {
        let there = grid.step(c, d);
        let back = grid.step(there.column, there.dir.opposite());
        prop_assert_eq!(back.column, c);
        prop_assert_eq!(back.dir, d.opposite());
    }

    /// A step moves to an adjacent cell: centres are about one block apart.
    #[test]
    fn step_lands_next_door((grid, c) in cell(), d in dir()) {
        let here = grid.column_direction(c);
        let there = grid.column_direction(grid.step(c, d).column);
        let cells = cells_apart(grid, here, there);
        prop_assert!((0.5..1.5).contains(&cells), "{cells} cells");
    }

    /// Four distinct neighbours, none of them self.
    #[test]
    fn neighbours_are_distinct((grid, c) in cell()) {
        let n = grid.neighbours(c);
        for i in 0..4 {
            prop_assert_ne!(n[i], c);
            for j in 0..i {
                prop_assert_ne!(n[i], n[j]);
            }
        }
    }

    /// direction and surface_point are inverse.
    #[test]
    fn direction_round_trip((grid, p) in point()) {
        let q = grid.surface_point(grid.direction(p));
        prop_assert_eq!(q.sector, p.sector);
        let tol = f64::from(grid.side()) * 1e-10;
        prop_assert!((q.u - p.u).abs() < tol && (q.v - p.v).abs() < tol, "{q:?} vs {p:?}");
    }

    /// Folding over a seam is continuous in world space: the overshoot lands
    /// where walking the same distance on the sphere would.
    #[test]
    fn wrap_is_continuous(
        grid in grid(),
        s in sector(),
        t in 0.0..1.0f64,
        share in 1e-6..0.5f64,
        edge in 0usize..4,
    ) {
        let side = f64::from(grid.side());
        // Away from the corners, where two folds would be needed.
        let along = side / 4.0 + t * side / 2.0;
        // The overshoot is a share of a face, so a coarse grid is not asked
        // to fold over several of them. A point exactly on the edge is still
        // its own sector's, so there is always some overshoot.
        let over = share * side;
        let (inside, outside) = match edge {
            0 => ((side, along), (side + over, along)),
            1 => ((0.0, along), (-over, along)),
            2 => ((along, side), (along, side + over)),
            _ => ((along, 0.0), (along, -over)),
        };
        let on_edge = grid.direction(SurfacePoint::new(s, inside.0, inside.1));
        let wrapped = grid.wrapped(SurfacePoint::new(s, outside.0, outside.1));
        prop_assert_ne!(wrapped.sector, s);
        let cells = cells_apart(grid, on_edge, grid.direction(wrapped));
        // Cells are near unit size at the seam, never exactly.
        prop_assert!((cells - over).abs() <= over * 0.5 + 1e-6, "{cells} vs {over}");
    }

    /// Tangents match finite differences of direction.
    #[test]
    fn tangents_match_finite_differences((grid, p) in point()) {
        let t = grid.tangents(p);
        // A share of a face, not an absolute step: on a coarse grid a fixed
        // step is a large part of the face and the difference stops being a
        // derivative. Small enough that truncation and cancellation both stay
        // under the tolerance.
        let e = f64::from(grid.side()) * 1e-5;
        let at = |u, v| grid.direction(SurfacePoint::new(p.sector, u, v));
        let fd_u = vec3::scale(vec3::sub(at(p.u + e, p.v), at(p.u - e, p.v)), 0.5 / e);
        let fd_v = vec3::scale(vec3::sub(at(p.u, p.v + e), at(p.u, p.v - e)), 0.5 / e);
        prop_assert!(vec3::length(vec3::sub(fd_u, t.du)) < 1e-9 * vec3::length(t.du).max(1.0));
        prop_assert!(vec3::length(vec3::sub(fd_v, t.dv)) < 1e-9 * vec3::length(t.dv).max(1.0));
        prop_assert!(vec3::dot(t.up, t.du).abs() < 1e-12 && vec3::dot(t.up, t.dv).abs() < 1e-12);
    }
}

/// Every legal size, every sector, every seam cell on a coarse stride, plus
/// the corners: the round trip property with no sampling luck involved.
#[test]
fn every_edge_round_trips() {
    for bits in 0..=MAX_BITS {
        let grid = Grid::new(bits).unwrap();
        let max = grid.max_coord();
        let stride = usize::from(max / 16).max(1);
        for s in Sector::ALL {
            for k in (0..=max).step_by(stride).chain([max]) {
                for c in [
                    Column::new(s, 0, k),
                    Column::new(s, max, k),
                    Column::new(s, k, 0),
                    Column::new(s, k, max),
                ] {
                    for d in Dir::ALL {
                        let there = grid.step(c, d);
                        let back = grid.step(there.column, there.dir.opposite());
                        assert_eq!(
                            (back.column, back.dir),
                            (c, d.opposite()),
                            "{bits} {c:?} {d:?}"
                        );
                    }
                }
            }
        }
    }
}

/// Each sector touches four others, never itself, never its opposite.
#[test]
fn each_sector_has_four_neighbours() {
    for bits in 1..=MAX_BITS {
        let grid = Grid::new(bits).unwrap();
        let max = grid.max_coord();
        let mid = max / 2;
        for s in Sector::ALL {
            let mut seen: Vec<Sector> = [
                grid.step(Column::new(s, max, mid), Dir::UPos),
                grid.step(Column::new(s, 0, mid), Dir::UNeg),
                grid.step(Column::new(s, mid, max), Dir::VPos),
                grid.step(Column::new(s, mid, 0), Dir::VNeg),
            ]
            .iter()
            .map(|step| step.column.sector)
            .collect();
            seen.sort();
            seen.dedup();
            assert_eq!(seen.len(), 4, "bits {bits}");
            assert!(!seen.contains(&s), "bits {bits}");
        }
    }
}

/// The numbers quoted in project.rs and ARCHITECTURE.md, at every size: the
/// warp is the same shape however wide the grid it is fed.
#[test]
fn block_distortion_is_bounded() {
    for bits in MIN_BITS..=MAX_BITS {
        let sphere = QuadSphere::new(bits).unwrap();
        let span = f64::from(sphere.blocks().side());
        let (mut min, mut max) = (f64::MAX, f64::MIN);
        for i in 0..=64 {
            for j in 0..=64 {
                let p = SurfacePoint::new(
                    Sector::ALL[0],
                    f64::from(i) / 64.0 * span,
                    f64::from(j) / 64.0 * span,
                );
                let t = sphere.blocks().tangents(p);
                for len in [vec3::length(t.du), vec3::length(t.dv)] {
                    let metres = len * sphere.radius_m();
                    min = min.min(metres);
                    max = max.max(metres);
                }
            }
        }
        println!(
            "bits {bits}: block edge {min:.4} m to {max:.4} m, ratio {:.4}",
            max / min
        );
        assert!(max / min < 1.5, "bits {bits}");
    }
}

/// The band is the human measure where the body can hold it, and what the core
/// leaves where it cannot. It never reaches the centre.
#[test]
fn band_never_eats_the_core() {
    for bits in MIN_BITS..=MAX_BITS {
        let sphere = QuadSphere::new(bits).unwrap();
        let band = f64::from(sphere.band_blocks());
        println!(
            "bits {bits}: radius {:.2} m, band +-{:.2} m",
            sphere.radius_m(),
            sphere.band_m()
        );
        assert!(band >= 1.0, "bits {bits}: no band at all");
        assert!(band <= sphere.radius_blocks() * 0.25 + 1.0, "bits {bits}");
        assert!(band <= 256.0, "bits {bits}");
    }
    // Today's planet keeps the number ARCHITECTURE quotes.
    assert_eq!(QuadSphere::new(16).unwrap().band_m(), 128.0);
}

/// The chunk grid is the block grid coarsened, so its seams are the block
/// grid's seams: a chunk that steps over an edge lands where a block would,
/// and nothing in the terrain has to know what a seam is.
#[test]
fn a_coarsened_grid_keeps_its_seams() {
    /// A chunk is `2^4` blocks a side (`voxel::CHUNK_BITS`).
    const CHUNK_BITS: u32 = 4;
    for bits in MIN_BITS..=MAX_BITS {
        let sphere = QuadSphere::new(bits).unwrap();
        let chunks = sphere
            .blocks()
            .coarsened(CHUNK_BITS)
            .expect("a body holds at least one chunk per sector side");
        assert_eq!(chunks.bits(), bits - CHUNK_BITS);
        let max = chunks.max_coord();
        for s in Sector::ALL {
            for k in [0, max / 2, max] {
                for c in [
                    Column::new(s, 0, k),
                    Column::new(s, max, k),
                    Column::new(s, k, 0),
                    Column::new(s, k, max),
                ] {
                    for d in Dir::ALL {
                        let there = chunks.step(c, d);
                        let back = chunks.step(there.column, there.dir.opposite());
                        assert_eq!((back.column, back.dir), (c, d.opposite()), "{bits} {c:?}");
                    }
                }
            }
        }
    }
}
