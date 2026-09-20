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
//! Each face carries a one texel gutter, filled at bake time from the ground
//! that continues past the sector edge. A sample at a seam then reads the same
//! ground from either side with no seam logic, and no branch, per sample.
//!
//! Nothing here allocates or reads a file: the bytes arrive from the shell,
//! already whole. Sampling is integer fetches and `+ - * /`, so a field world
//! is as deterministic as a generated one.

use topology::{BLOCK_M, SECTOR_SIDE, SurfacePoint, Vec3};

/// Fields start with this, so a wrong file is refused instead of decoded.
const MAGIC: [u8; 8] = *b"PLFIELD1";
/// Magic, id, side, levels.
const HEADER: usize = 8 + 32 + 4 + 4;
/// Metres of spread in one step of the ruggedness byte. Nearly all ground
/// ranges under a few hundred metres inside a texel, so the step is small and
/// the top of the byte is the Himalaya, not headroom nobody uses.
const RUGGED_STEP: f64 = 16.0;
/// Metres along one sector side: the width of a face on our planet.
const FACE_M: f64 = SECTOR_SIDE as f64 * BLOCK_M;

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
    /// Reads a baked field. The id is the bake's, not recomputed here: what
    /// guarantees the bytes is the content addressed store they came from.
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
        Ok(Field { bytes, id, levels })
    }

    /// What the recipe names this field by.
    pub fn id(&self) -> [u8; 32] {
        self.id
    }

    /// Metres of ground one texel of the finest level covers.
    pub fn texel_m(&self) -> f64 {
        FACE_M / f64::from(self.levels[0].side)
    }

    /// The ground under a direction, as a mesh sampled every `footprint_m`
    /// metres should see it: levels finer than the mesh are blended out, the
    /// way `band` fades an octave, so no level change ever pops.
    pub fn sample(&self, d: Vec3, footprint_m: f64) -> Ground {
        // Levels double in size, so the level that matches a footprint is its
        // log. Below the finest level there is nothing coarser to fall back
        // on: the generator's noise takes over there.
        let wanted = libm::log2((footprint_m / self.texel_m()).max(1.0));
        let last = (self.levels.len() - 1) as f64;
        let coarse = libm::floor(wanted).min(last);
        let blend = (wanted - coarse).clamp(0.0, 1.0);
        let near = self.level(coarse as usize, d);
        if blend <= 0.0 {
            return near;
        }
        let far = self.level(coarse as usize + 1, d);
        Ground {
            elevation_m: near.elevation_m + (far.elevation_m - near.elevation_m) * blend,
            ruggedness_m: near.ruggedness_m + (far.ruggedness_m - near.ruggedness_m) * blend,
        }
    }

    /// Bilinear fetch inside one level. The gutter makes the face edge an
    /// ordinary texel, so nothing here knows what a seam is.
    fn level(&self, index: usize, d: Vec3) -> Ground {
        let level = self.levels[index.min(self.levels.len() - 1)];
        let point = SurfacePoint::from_direction(d);
        let span = f64::from(level.side) / f64::from(SECTOR_SIDE);
        // Texel `t` has its centre at `t + 0.5` across the face and lives at
        // `t + 1` in the array: adding 0.5 puts a texel centre on an integer.
        let x = point.u * span + 0.5;
        let y = point.v * span + 0.5;
        let last = (level.side + 1) as f64;
        let x0 = libm::floor(x).clamp(0.0, last - 1.0);
        let y0 = libm::floor(y).clamp(0.0, last - 1.0);
        let (fx, fy) = (x - x0, y - y0);
        let face = level.offset + point.sector.index() * level.face_bytes();
        let stride = level.stride();
        let at = |ox: f64, oy: f64| {
            let a = (x0 + ox) as usize + (y0 + oy) as usize * stride;
            (self.elevation(face, a), self.ruggedness(face, stride, a))
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

    fn elevation(&self, face: usize, texel: usize) -> f64 {
        let at = face + texel * 2;
        f64::from(i16::from_le_bytes([self.bytes[at], self.bytes[at + 1]]))
    }

    fn ruggedness(&self, face: usize, stride: usize, texel: usize) -> f64 {
        let at = face + stride * stride * 2 + texel;
        f64::from(self.bytes[at]) * RUGGED_STEP
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A field whose every texel of level `l` holds `elevation(l)`, so a
    /// sample says which level it read and whether it read it in the right
    /// place. The gutter holds the same value as the face, which is what a
    /// bake produces for ground that is constant across a seam.
    fn constant(side: u32, levels: u32, elevation: impl Fn(u32) -> i16) -> Field {
        let mut bytes = Vec::from(MAGIC);
        bytes.extend([0u8; 32]);
        bytes.extend(side.to_le_bytes());
        bytes.extend(levels.to_le_bytes());
        for level in 0..levels {
            let stride = (side >> level) as usize + 2;
            let plane = stride * stride;
            for _ in 0..6 {
                for _ in 0..plane {
                    bytes.extend(elevation(level).to_le_bytes());
                }
                bytes.extend(core::iter::repeat_n(level as u8, plane));
            }
        }
        Field::parse(bytes).expect("a field this test just wrote")
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

    #[test]
    fn every_direction_reads_the_level_its_footprint_asks_for() {
        let field = constant(64, 4, |level| 100 * (level as i16 + 1));
        let texel_m = field.texel_m();
        for (footprint_m, want) in [
            (0.0, 100.0),
            (texel_m, 100.0),
            (texel_m * 2.0, 200.0),
            (texel_m * 8.0, 400.0),
            // Past the coarsest level there is nothing coarser to read.
            (texel_m * 1024.0, 400.0),
        ] {
            for d in directions() {
                let ground = field.sample(d, footprint_m);
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
        let ground = field.sample([1.0, 0.0, 0.0], field.texel_m() * 3.0);
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
        let ground = field.sample([0.0, 1.0, 0.0], 0.0);
        assert_eq!(ground.ruggedness_m, 0.0);
        let coarse = field.sample([0.0, 1.0, 0.0], field.texel_m() * 2.0);
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
            let here = field.sample(d, 0.0).elevation_m;
            if let Some(before) = previous {
                assert!(libm::fabs(here - before) < 1e-9, "{before} -> {here}");
            }
            previous = Some(here);
        }
    }
}
