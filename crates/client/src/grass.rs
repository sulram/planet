//! Cosmetic vegetation from existing terrain samples, with no generator calls.
//! A tuft is identified by integers (sector, tier, tier cell), never by the
//! patch that happens to carry it, so subdivision does not move it.

use glam::{DVec3, Vec3};
use scene::{GRASS_TIER_0_REACH_M, GrassInstance, PATCH_GRID, TerrainVertex};
use worldgen::{Material, Sample};

/// Tier `k` holds one tuft per cell of `2^k` half blocks and is seen out to
/// `GRASS_TIER_0_REACH_M * 2^k`: spacing doubles as reach doubles, so the
/// density on screen stays level from the feet to the last tier's reach.
pub const TIERS: u32 = 6;

pub struct Patch {
    pub seed: u64,
    pub sector: u32,
    pub cell: [u32; 2],
    pub depth: u32,
    /// Blocks per sector side of the body this patch is on. A world's size is
    /// a recipe value (49), so a tier's cell is counted against it and not
    /// against a constant.
    pub sector_side: u32,
}

fn hash(seed: u64, sector: u32, tier: u32, u: u32, v: u32) -> u32 {
    let mut h = seed
        ^ (u64::from(sector) << 48)
        ^ (u64::from(tier) << 40)
        ^ (u64::from(u) << 20)
        ^ u64::from(v);
    h = (h ^ (h >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    h = (h ^ (h >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    (h ^ (h >> 31)) as u32
}

/// The finest tier a patch carries. A patch `j` subdivisions short of the
/// finest is drawn only from about `27 * 2^j` m away (it would have split
/// otherwise), beyond the reach of every tier up to `j`. The finest patch is
/// drawn underfoot and carries them all.
fn first_tier(side: u32) -> u32 {
    match (side / PATCH_GRID).trailing_zeros() {
        0 => 0,
        j => j + 1,
    }
}

/// Tufts of a patch, farthest reaching tier first: a renderer draws a prefix.
/// Roots are interpolated on the triangles being drawn, so they sit on them.
pub fn build(
    patch: Patch,
    origin: DVec3,
    vertices: &[TerrainVertex],
    samples: &[(DVec3, Sample)],
) -> Vec<GrassInstance> {
    let side = patch.sector_side >> patch.depth;
    if side < PATCH_GRID {
        return Vec::new();
    }
    let g = PATCH_GRID as usize;
    let half_blocks = side * 2;
    let mut grass = Vec::new();
    for tier in (first_tier(side)..TIERS).rev() {
        let cells = half_blocks >> tier;
        let first = [patch.cell[0] * cells, patch.cell[1] * cells];
        let reach_m = GRASS_TIER_0_REACH_M * (1u32 << tier) as f32;
        for cv in 0..cells {
            for cu in 0..cells {
                let h = hash(patch.seed, patch.sector, tier, first[0] + cu, first[1] + cv);
                let random = |shift: u32| ((h >> shift) & 255) as f32 / 255.0;
                let scale = g as f32 / cells as f32;
                // Clear of the rim: a root never lands on a neighbour's triangle.
                let x = (cu as f32 + 0.05 + random(0) * 0.9) * scale;
                let y = (cv as f32 + 0.05 + random(8) * 0.9) * scale;
                let (i, j) = ((x as usize).min(g - 1), (y as usize).min(g - 1));
                // Require the four corners to be meadow: no tuft on a beach or rock.
                let corners = [(i, j), (i + 1, j), (i, j + 1), (i + 1, j + 1)];
                if !corners.iter().all(|&(a, b)| {
                    let s = samples[(b + 1) * (g + 3) + a + 1].1;
                    s.height_m > 1.5 && s.material == Material::Grass
                }) {
                    continue;
                }
                let (fx, fy) = (x - i as f32, y - j as f32);
                let (ids, weights) = if fx + fy <= 1.0 {
                    (
                        [j * (g + 1) + i, j * (g + 1) + i + 1, (j + 1) * (g + 1) + i],
                        [1.0 - fx - fy, fx, fy],
                    )
                } else {
                    (
                        [
                            j * (g + 1) + i + 1,
                            (j + 1) * (g + 1) + i + 1,
                            (j + 1) * (g + 1) + i,
                        ],
                        [1.0 - fy, fx + fy - 1.0, 1.0 - fx],
                    )
                };
                let mut root = Vec3::ZERO;
                let mut normal = Vec3::ZERO;
                for (id, w) in ids.into_iter().zip(weights) {
                    root += Vec3::from(vertices[id].position) * w;
                    normal += Vec3::from(vertices[id].normal) * w;
                }
                let up = (origin + root.as_dvec3()).normalize().as_vec3();
                let normal = normal.normalize();
                if normal.dot(up) < 0.85 {
                    continue;
                }
                let tint = random(16);
                grass.push(GrassInstance {
                    root: (root - up * 0.015).to_array(),
                    height: 0.55 + tint * 0.50,
                    normal: normal.to_array(),
                    reach_m,
                    color: [0.10 + tint * 0.05, 0.20 + tint * 0.09, 0.035 + tint * 0.02],
                    angle: random(24) * core::f32::consts::TAU,
                });
            }
        }
    }
    grass
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The reference body, which is what these fixtures were measured on.
    const SIDE: u32 = 1 << 16;

    fn fixture(depth: u32, cell: [u32; 2], material: Material) -> (DVec3, Vec<GrassInstance>) {
        let side = f64::from(SIDE >> depth) * 0.5;
        let origin = DVec3::new(
            21_000.0,
            f64::from(cell[0]) * side,
            f64::from(cell[1]) * side,
        );
        let mut vertices = Vec::new();
        for j in 0..=PATCH_GRID {
            for i in 0..=PATCH_GRID {
                vertices.push(TerrainVertex {
                    position: [
                        0.0,
                        (f64::from(i) * side / f64::from(PATCH_GRID)) as f32,
                        (f64::from(j) * side / f64::from(PATCH_GRID)) as f32,
                    ],
                    normal: [1.0, 0.0, 0.0],
                    color: [104, 138, 70, 0],
                });
            }
        }
        let samples = vec![
            (
                origin,
                Sample {
                    height_m: 100.0,
                    material
                }
            );
            ((PATCH_GRID + 3) * (PATCH_GRID + 3)) as usize
        ];
        let grass = build(
            Patch {
                seed: 17,
                sector: 2,
                cell,
                depth,
                sector_side: SIDE,
            },
            origin,
            &vertices,
            &samples,
        );
        (origin, grass)
    }

    /// World root, height and reach of every tuft from `tier` up, in one order.
    fn shared(depth: u32, cells: &[[u32; 2]], tier: u32) -> Vec<(DVec3, f32, f32)> {
        let reach_m = GRASS_TIER_0_REACH_M * (1u32 << tier) as f32;
        let mut tufts = Vec::new();
        for &cell in cells {
            let (origin, grass) = fixture(depth, cell, Material::Grass);
            assert!(grass.windows(2).all(|w| w[0].reach_m >= w[1].reach_m));
            tufts.extend(
                grass
                    .iter()
                    .filter(|g| g.reach_m >= reach_m)
                    .map(|g| (origin + Vec3::from(g.root).as_dvec3(), g.height, g.reach_m)),
            );
        }
        // Millimetres: a root differs by an f32 ulp between a patch and its child.
        tufts.sort_by_key(|t| {
            (
                t.2 as i64,
                (t.0.y * 1000.0).round() as i64,
                (t.0.z * 1000.0).round() as i64,
            )
        });
        tufts
    }

    #[test]
    fn subdivision_keeps_roots_height_and_tier() {
        // Depth 10 carries tiers 2 and up; its four children carry them too.
        let parent = shared(10, &[[0, 0]], 2);
        let children = shared(11, &[[0, 0], [1, 0], [0, 1], [1, 1]], 2);
        assert_eq!(parent.len(), 1024 + 256 + 64 + 16);
        assert_eq!(parent.len(), children.len());
        for (a, b) in parent.iter().zip(&children) {
            assert!(a.0.distance(b.0) < 0.0001, "{a:?} {b:?}");
            assert_eq!((a.1, a.2), (b.1, b.2));
        }
    }

    #[test]
    fn a_tuft_has_one_owner() {
        let tufts = shared(11, &[[0, 0], [1, 0], [0, 1], [1, 1]], 0);
        for pair in tufts.windows(2) {
            assert!(pair[0].2 != pair[1].2 || pair[0].0.distance(pair[1].0) > 0.001);
        }
    }

    #[test]
    fn bare_ground_has_no_grass_and_distant_patches_carry_only_far_tiers() {
        for material in [
            Material::Forest,
            Material::Sand,
            Material::Water,
            Material::Rock,
            Material::Snow,
            Material::Regolith,
        ] {
            assert!(fixture(11, [0, 0], material).1.is_empty());
        }
        let (_, far) = fixture(7, [0, 0], Material::Grass);
        assert_eq!(far.len(), 1024);
        let last = GRASS_TIER_0_REACH_M * (1u32 << (TIERS - 1)) as f32;
        assert!(far.iter().all(|g| g.reach_m == last));
        assert!(fixture(6, [0, 0], Material::Grass).1.is_empty());
    }
}
