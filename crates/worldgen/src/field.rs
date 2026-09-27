//! The field: ground shape baked into a cube map, one face per sector.
//!
//! A world whose recipe names a field is shaped by it above the texel and by
//! the generator's own noise below, the same way [`crate::noise::band`] fades
//! an octave finer than the mesh that asks. The field is the low frequencies
//! of some real body; the seed still owns everything smaller than a texel, so
//! one field is a family of worlds, not a world.
//!
//! Two channels per texel, because a level that only averaged would flatten
//! the mountains it covers:
//! - `elevation`, metres over the source body's sea level, as `i16`;
//! - `ruggedness`, how far elevation ranges inside one *finest* texel, in
//!   steps of `RUGGED_STEP` metres. What a level loses becomes roughness, as
//!   the sea's waves do (DECISIONS 43), and it is what tells a cordillera
//!   from a plateau of the same height. Coarse levels average it rather than
//!   accumulate it, so it stays the same measure of the same ground at every
//!   level: a silhouette must not change because the camera moved away.
//!
//! Each face carries a one texel gutter: a slot the bake leaves empty and
//! [`Field::parse`] fills once from the neighbouring face. The texel past a
//! sector edge is the neighbour's first texel, folded across the seam in
//! address space the way a step across the edge is ([`Grid::wrapped`]), and
//! where three faces meet the corner texel is the mean of their three corner
//! texels. A sample at a seam then reads the same ground from either side
//! with no seam logic, and no branch, per sample (DECISIONS 71).
//!
//! Nothing here allocates or reads a file: the bytes arrive from the shell,
//! already whole, and the gutter is filled in place. Sampling is integer
//! fetches and `+ - * /`, so a field world is as deterministic as a
//! generated one.

use topology::{BLOCK_M, Grid, QuadSphere, Sector, SurfacePoint, Vec3};

/// Fields start with this, so a wrong file is refused instead of decoded.
const MAGIC: [u8; 8] = *b"PLFIELD1";
/// Magic, id, side, levels.
const HEADER: usize = 8 + 32 + 4 + 4;
/// Metres of spread in one step of the ruggedness byte. Nearly all ground
/// ranges under a few hundred metres inside a texel, so the step is small and
/// the top of the byte is the Himalaya, not headroom nobody uses.
const RUGGED_STEP: f64 = 16.0;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum FieldError {
    /// The bytes do not start with the magic.
    NotAField,
    /// Faces are square and a power of two, between 16 and 8192.
    UnsupportedSide(u32),
    /// The header promises more levels or faces than the bytes hold.
    Truncated,
}

impl core::fmt::Display for FieldError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            FieldError::NotAField => write!(f, "not a field: wrong magic"),
            FieldError::UnsupportedSide(side) => {
                write!(
                    f,
                    "unsupported face side {side}: want a power of two in 16..=8192"
                )
            }
            FieldError::Truncated => write!(f, "field is shorter than its header promises"),
        }
    }
}

impl std::error::Error for FieldError {}

/// One mip level: the same ground, at half the texels of the one before.
#[derive(Clone, Copy, Debug)]
struct Level {
    /// Texels per face side, gutter not counted.
    side: u32,
    /// Byte offset of this level's face 0.
    offset: usize,
}

impl Level {
    /// Texels per face side with the gutter: the row stride of a face.
    fn stride(self) -> usize {
        self.side as usize + 2
    }

    /// Bytes one face takes: both channels.
    fn face_bytes(self) -> usize {
        self.stride() * self.stride() * 3
    }

    /// Byte offset of a sector's face in this level.
    fn face(self, sector: Sector) -> usize {
        self.offset + sector.index() * self.face_bytes()
    }

    /// The array coordinates of a face's gutter: its outermost ring.
    fn gutter(self) -> impl Iterator<Item = (usize, usize)> {
        let last = self.side as usize + 1;
        (0..=last)
            .flat_map(move |x| [(x, 0), (x, last)])
            .chain((1..last).flat_map(move |y| [(0, y), (last, y)]))
    }
}

/// The ground one texel of the field describes.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Ground {
    /// Metres over the source body's sea level. The zero is its coastline.
    pub elevation_m: f64,
    /// How far elevation ranges inside what this sample covers, metres.
    pub ruggedness_m: f64,
}

/// A baked cube map of ground, ready to sample.
#[derive(Clone, Debug)]
pub struct Field {
    bytes: Vec<u8>,
    id: [u8; 32],
    levels: Vec<Level>,
}

impl Field {
    /// Reads a baked field and fills its gutters. The id is the bake's, not
    /// recomputed here: what guarantees the bytes is the content addressed
    /// store they came from.
    pub fn parse(bytes: Vec<u8>) -> Result<Field, FieldError> {
        if bytes.len() < HEADER || bytes[..8] != MAGIC {
            return Err(FieldError::NotAField);
        }
        let mut id = [0u8; 32];
        id.copy_from_slice(&bytes[8..40]);
        let word = |at: usize| {
            u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
        };
        let side = word(40);
        let count = word(44);
        if !(16..=8192).contains(&side) || !side.is_power_of_two() {
            return Err(FieldError::UnsupportedSide(side));
        }
        if count == 0 || count > side.trailing_zeros() {
            return Err(FieldError::Truncated);
        }
        let mut levels = Vec::with_capacity(count as usize);
        let mut offset = HEADER;
        for index in 0..count {
            let level = Level {
                side: side >> index,
                offset,
            };
            offset += 6 * level.face_bytes();
            levels.push(level);
        }
        if bytes.len() < offset {
            return Err(FieldError::Truncated);
        }
        let mut field = Field { bytes, id, levels };
        field.fold_seams();
        Ok(field)
    }

    /// Fills every gutter texel from the face across its seam. A gutter
    /// texel's centre lies half a texel past the edge; folded in address
    /// space that is the centre of the neighbour's first texel, so the pair
    /// each face interpolates along an edge is the same pair. Projecting
    /// the gutter's direction onto the neighbour instead would shift the
    /// coordinate along the edge, and each face would read its own boundary.
    /// Where three faces meet there is no fourth texel: the corner takes the
    /// mean of the three, the same number from whichever face reads it.
    fn fold_seams(&mut self) {
        for level in self.levels.clone() {
            let grid = Grid::new(level.side.trailing_zeros()).expect("a face side is a grid");
            let side = f64::from(level.side);
            let stride = level.stride();
            let last = stride - 1;
            // The face and texel holding the texel centred at `(u, v)` of a
            // sector, in texel units, once folded into range.
            let source = |sector: Sector, u: f64, v: f64| {
                let column = grid.column_of(grid.wrapped(SurfacePoint::new(sector, u, v)));
                let texel = (column.v as usize + 1) * stride + column.u as usize + 1;
                (level.face(column.sector), texel)
            };
            for sector in Sector::ALL {
                let face = level.face(sector);
                for (x, y) in level.gutter() {
                    let (u, v) = (x as f64 - 0.5, y as f64 - 0.5);
                    let value = if (x == 0 || x == last) && (y == 0 || y == last) {
                        let (cu, cv) = (u.clamp(0.5, side - 0.5), v.clamp(0.5, side - 0.5));
                        let taps = [(cu, cv), (u, cv), (cu, v)].map(|(u, v)| {
                            let (face, texel) = source(sector, u, v);
                            self.texel(face, stride, texel)
                        });
                        let e = taps.iter().map(|t| i32::from(t.0)).sum::<i32>();
                        let r = taps.iter().map(|t| i32::from(t.1)).sum::<i32>();
                        // The nearest third: a third never lands on a half.
                        ((e + 1).div_euclid(3) as i16, (r + 1).div_euclid(3) as u8)
                    } else {
                        let (face, texel) = source(sector, u, v);
                        self.texel(face, stride, texel)
                    };
                    self.set_texel(face, stride, y * stride + x, value);
                }
            }
        }
    }

    /// What the recipe names this field by.
    pub fn id(&self) -> [u8; 32] {
        self.id
    }

    /// Metres of ground one texel of the finest level covers, on the body
    /// the field is stretched over. A field is a shape, so the same bytes
    /// cover a small world at a proportionally finer grain.
    pub fn texel_m(&self, sphere: QuadSphere) -> f64 {
        f64::from(sphere.blocks().side()) * BLOCK_M / f64::from(self.levels[0].side)
    }

    /// The ground under a direction, as a mesh sampled every `footprint_m`
    /// metres should see it: levels finer than the mesh are blended out, the
    /// way `band` fades an octave, so no level change ever pops.
    pub fn sample(&self, sphere: QuadSphere, d: Vec3, footprint_m: f64) -> Ground {
        // Levels double in size, so the level that matches a footprint is its
        // log. Below the finest level there is nothing coarser to fall back
        // on: the generator's noise takes over there.
        let wanted = libm::log2((footprint_m / self.texel_m(sphere)).max(1.0));
        let last = (self.levels.len() - 1) as f64;
        let coarse = libm::floor(wanted).min(last);
        let blend = (wanted - coarse).clamp(0.0, 1.0);
        let near = self.level(sphere, coarse as usize, d);
        if blend <= 0.0 {
            return near;
        }
        let far = self.level(sphere, coarse as usize + 1, d);
        Ground {
            elevation_m: near.elevation_m + (far.elevation_m - near.elevation_m) * blend,
            ruggedness_m: near.ruggedness_m + (far.ruggedness_m - near.ruggedness_m) * blend,
        }
    }

    /// Bilinear fetch inside one level. The gutter makes the face edge an
    /// ordinary texel, so nothing here knows what a seam is.
    fn level(&self, sphere: QuadSphere, index: usize, d: Vec3) -> Ground {
        let level = self.levels[index.min(self.levels.len() - 1)];
        let point = sphere.blocks().surface_point(d);
        let span = f64::from(level.side) / f64::from(sphere.blocks().side());
        // Texel `t` has its centre at `t + 0.5` across the face and lives at
        // `t + 1` in the array: adding 0.5 puts a texel centre on an integer.
        let x = point.u * span + 0.5;
        let y = point.v * span + 0.5;
        let last = (level.side + 1) as f64;
        let x0 = libm::floor(x).clamp(0.0, last - 1.0);
        let y0 = libm::floor(y).clamp(0.0, last - 1.0);
        let (fx, fy) = (x - x0, y - y0);
        let face = level.face(point.sector);
        let stride = level.stride();
        let at = |ox: f64, oy: f64| {
            let (e, r) = self.texel(
                face,
                stride,
                (x0 + ox) as usize + (y0 + oy) as usize * stride,
            );
            (f64::from(e), f64::from(r) * RUGGED_STEP)
        };
        let (e00, r00) = at(0.0, 0.0);
        let (e10, r10) = at(1.0, 0.0);
        let (e01, r01) = at(0.0, 1.0);
        let (e11, r11) = at(1.0, 1.0);
        let mix = |a: f64, b: f64, c: f64, e: f64| {
            let top = a + (b - a) * fx;
            let bottom = c + (e - c) * fx;
            top + (bottom - top) * fy
        };
        Ground {
            elevation_m: mix(e00, e10, e01, e11),
            ruggedness_m: mix(r00, r10, r01, r11),
        }
    }

    /// Both channels of one texel, as stored.
    fn texel(&self, face: usize, stride: usize, texel: usize) -> (i16, u8) {
        let at = face + texel * 2;
        (
            i16::from_le_bytes([self.bytes[at], self.bytes[at + 1]]),
            self.bytes[face + stride * stride * 2 + texel],
        )
    }

    fn set_texel(&mut self, face: usize, stride: usize, texel: usize, (e, r): (i16, u8)) {
        let at = face + texel * 2;
        self.bytes[at..at + 2].copy_from_slice(&e.to_le_bytes());
        self.bytes[face + stride * stride * 2 + texel] = r;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What a gutter holds before the parser fills it. No test bakes this
    /// value, so a texel still holding it was never filled.
    const EMPTY: (i16, u8) = (-10_000, 255);

    /// A field whose interior texels hold what `texel` says of their level,
    /// sector and array coordinates, `1..=side` inside the gutter. The gutter
    /// is left as the bake leaves it: empty.
    fn baked(
        side: u32,
        levels: u32,
        texel: impl Fn(u32, usize, usize, usize) -> (i16, u8),
    ) -> Field {
        let mut bytes = Vec::from(MAGIC);
        bytes.extend([0u8; 32]);
        bytes.extend(side.to_le_bytes());
        bytes.extend(levels.to_le_bytes());
        for level in 0..levels {
            let side = (side >> level) as usize;
            let stride = side + 2;
            for sector in 0..6 {
                let mut elevation = Vec::with_capacity(stride * stride * 2);
                let mut ruggedness = Vec::with_capacity(stride * stride);
                for y in 0..stride {
                    for x in 0..stride {
                        let inside = (1..=side).contains(&x) && (1..=side).contains(&y);
                        let (e, r) = if inside {
                            texel(level, sector, x, y)
                        } else {
                            EMPTY
                        };
                        elevation.extend(e.to_le_bytes());
                        ruggedness.push(r);
                    }
                }
                bytes.extend(elevation);
                bytes.extend(ruggedness);
            }
        }
        Field::parse(bytes).expect("a field this test just wrote")
    }

    /// A field whose every texel of level `l` holds `elevation(l)`, so a
    /// sample says which level it read and whether it read it in the right
    /// place.
    fn constant(side: u32, levels: u32, elevation: impl Fn(u32) -> i16) -> Field {
        baked(side, levels, |level, _, _, _| {
            (elevation(level), level as u8)
        })
    }

    /// Unequal faces and slopes, so a seam read from the wrong texel, an edge
    /// folded the wrong way round or a corner taken from one face alone all
    /// show. Constant faces cannot tell a crack from a plain.
    fn sloped() -> Field {
        baked(64, 3, |level, sector, x, y| {
            (
                (sector * 1000 + x * 3 + y * 7 + level as usize * 200) as i16,
                (sector * 20 + x + y) as u8,
            )
        })
    }

    /// Directions into every sector, none of them on a seam.
    fn directions() -> Vec<Vec3> {
        let mut out = Vec::new();
        for i in -3i32..=3 {
            for j in -3i32..=3 {
                for k in -3i32..=3 {
                    if (i, j, k) != (0, 0, 0) {
                        let v = [f64::from(i), f64::from(j), f64::from(k)];
                        let len = libm::sqrt(v[0] * v[0] + v[1] * v[1] + v[2] * v[2]);
                        out.push([v[0] / len, v[1] / len, v[2] / len]);
                    }
                }
            }
        }
        out
    }

    #[test]
    fn a_wrong_file_is_refused_instead_of_decoded() {
        assert_eq!(
            Field::parse(vec![0; 64]).unwrap_err(),
            FieldError::NotAField
        );
        let mut short = Vec::from(MAGIC);
        short.extend([0u8; 32]);
        short.extend(1024u32.to_le_bytes());
        short.extend(4u32.to_le_bytes());
        assert_eq!(Field::parse(short).unwrap_err(), FieldError::Truncated);
    }

    /// The body a field is stretched over in these tests: today's planet.
    fn planet() -> QuadSphere {
        QuadSphere::new(topology::MAX_BITS).expect("a legal size")
    }

    #[test]
    fn every_direction_reads_the_level_its_footprint_asks_for() {
        let field = constant(64, 4, |level| 100 * (level as i16 + 1));
        let sphere = planet();
        let texel_m = field.texel_m(sphere);
        for (footprint_m, want) in [
            (0.0, 100.0),
            (texel_m, 100.0),
            (texel_m * 2.0, 200.0),
            (texel_m * 8.0, 400.0),
            // Past the coarsest level there is nothing coarser to read.
            (texel_m * 1024.0, 400.0),
        ] {
            for d in directions() {
                let ground = field.sample(sphere, d, footprint_m);
                assert!(
                    libm::fabs(ground.elevation_m - want) < 1e-9,
                    "{d:?} at {footprint_m} m: want {want}, got {}",
                    ground.elevation_m
                );
            }
        }
    }

    #[test]
    fn a_footprint_between_levels_blends_them() {
        let field = constant(64, 3, |level| 100 * (level as i16 + 1));
        let sphere = planet();
        let ground = field.sample(sphere, [1.0, 0.0, 0.0], field.texel_m(sphere) * 3.0);
        // log2(3) is 1.585: most of the way from level 1 to level 2.
        assert!(
            (258.0..259.0).contains(&ground.elevation_m),
            "{}",
            ground.elevation_m
        );
    }

    #[test]
    fn ruggedness_comes_back_in_metres() {
        let field = constant(32, 2, |_| 0);
        let sphere = planet();
        let ground = field.sample(sphere, [0.0, 1.0, 0.0], 0.0);
        assert_eq!(ground.ruggedness_m, 0.0);
        let coarse = field.sample(sphere, [0.0, 1.0, 0.0], field.texel_m(sphere) * 2.0);
        assert_eq!(coarse.ruggedness_m, RUGGED_STEP);
    }

    #[test]
    fn a_seam_is_an_ordinary_texel() {
        // Walking across a sector edge must not jump: the gutter carries the
        // ground from the other side, so the two faces agree there.
        let field = constant(64, 2, |_| 500);
        let step = 1e-4;
        let mut previous = None;
        for i in -20..=20 {
            // A path that crosses the +X / +Z seam.
            let angle = core::f64::consts::FRAC_PI_4 + f64::from(i) * step;
            let d = [libm::cos(angle), 0.0, libm::sin(angle)];
            let here = field.sample(planet(), d, 0.0).elevation_m;
            if let Some(before) = previous {
                assert!(libm::fabs(here - before) < 1e-9, "{before} -> {here}");
            }
            previous = Some(here);
        }
    }

    #[test]
    fn the_parser_fills_every_gutter_texel_and_only_those() {
        let field = sloped();
        for level in &field.levels {
            let stride = level.stride();
            for sector in Sector::ALL {
                let face = level.face(sector);
                for y in 0..stride {
                    for x in 0..stride {
                        let texel = field.texel(face, stride, y * stride + x);
                        let inside = (1..stride - 1).contains(&x) && (1..stride - 1).contains(&y);
                        assert_ne!(texel, EMPTY, "{sector:?} ({x}, {y}) side {}", level.side);
                        if inside {
                            let want = (
                                (sector.index() * 1000 + x * 3 + y * 7) as i16
                                    + (64 / level.side as i16).trailing_zeros() as i16 * 200,
                                (sector.index() * 20 + x + y) as u8,
                            );
                            assert_eq!(texel, want, "{sector:?} ({x}, {y}) side {}", level.side);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn unequal_faces_meet_at_every_edge_and_corner_at_every_mip() {
        let field = sloped();
        let sphere = planet();
        for footprint in [0.0, 1.0, 1.5, 2.0, 3.0, 4.0, 16.0] {
            for a in 0..3 {
                for b in a + 1..3 {
                    let c = 3 - a - b;
                    for sa in [-1.0, 1.0] {
                        for sb in [-1.0, 1.0] {
                            for along in [-1.0, -0.9999, -0.75, -0.01, 0.0, 0.37, 0.9999, 1.0] {
                                let mut d = [0.0; 3];
                                d[a] = sa;
                                d[b] = sb;
                                d[c] = along;
                                // A hair onto one face, then a hair onto the
                                // other: the same ground from both.
                                let sample = |axis: usize| {
                                    let mut p = d;
                                    p[axis] *= 1.0 + 1e-10;
                                    field.sample(sphere, p, footprint * field.texel_m(sphere))
                                };
                                let (left, right) = (sample(a), sample(b));
                                assert!(
                                    (left.elevation_m - right.elevation_m).abs() < 1e-4
                                        && (left.ruggedness_m - right.ruggedness_m).abs() < 1e-4,
                                    "{d:?}, footprint {footprint}: {left:?} != {right:?}"
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn interior_texel_centres_keep_the_baked_values() {
        let field = sloped();
        let sphere = planet();
        for (mip, level) in field.levels.iter().enumerate() {
            let grid = Grid::new(level.side.trailing_zeros()).unwrap();
            for sector in Sector::ALL {
                for y in 0..level.side {
                    for x in 0..level.side {
                        let d = grid.direction(SurfacePoint::new(
                            sector,
                            f64::from(x) + 0.5,
                            f64::from(y) + 0.5,
                        ));
                        let ground =
                            field.sample(sphere, d, field.texel_m(sphere) * (1 << mip) as f64);
                        let want = (sector.index() * 1000
                            + (x as usize + 1) * 3
                            + (y as usize + 1) * 7
                            + mip * 200) as f64;
                        assert!((ground.elevation_m - want).abs() < 1e-8);
                    }
                }
            }
        }
    }

    #[test]
    fn a_corner_averages_its_three_faces() {
        let ground = sloped().sample(planet(), [1.0; 3], 0.0);
        assert!((ground.elevation_m - 2640.0).abs() < 1e-9, "{ground:?}");
        assert!((ground.ruggedness_m - 2688.0).abs() < 1e-9, "{ground:?}");
    }

    /// Seam sampling, frozen: a change here moves ground under every field
    /// world's feet at its sector edges, so it happens on purpose.
    #[test]
    fn field_seam_samples_are_frozen() {
        let field = sloped();
        let mut hash = 0xcbf2_9ce4_8422_2325u64;
        for i in -3i32..=3 {
            for j in -3i32..=3 {
                for k in -3i32..=3 {
                    if (i, j, k) == (0, 0, 0) {
                        continue;
                    }
                    let d = [f64::from(i), f64::from(j), f64::from(k)];
                    for footprint in [0.0, 512.0, 768.0, 1024.0, 1536.0, 2048.0] {
                        let ground = field.sample(planet(), d, footprint);
                        for value in [ground.elevation_m, ground.ruggedness_m] {
                            for byte in value.to_bits().to_le_bytes() {
                                hash = (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3);
                            }
                        }
                    }
                }
            }
        }
        assert_eq!(
            hash, 0xba4b_c69c_5175_03a4,
            "field seam sampling changed: {hash:#x}"
        );
    }
}
