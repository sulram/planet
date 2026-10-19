//! A part: a placed thing that is no cell (DECISIONS 111). A cube first; a
//! model, a picture, a light are shapes of the same thing.
//!
//! A part knows no sphere (CLAUDE.md, How it grows). It stands in a flat
//! frame, blocks along `x` and `y` and up `z`, the frame of the seat it is
//! on, and seating it on a body is `seat`'s job: the frame is the seat's
//! tangent frame at the anchor, and a part is rigid in it. Everything here
//! is said in that frame and in blocks: where a line meets a part, what a
//! body standing beside one has under its feet and over its head.
//!
//! A part's own space is the unit box: `x` and `y` from `-0.5` to `0.5`,
//! `z` from `0` to `1`, its anchor the middle of its bottom. Its pose turns
//! and stretches that box, so a cube of scale 1 is one cell, and a model's
//! unit is one of its own voxels.

use glam::{DAffine3, DQuat, DVec3};

/// The name of a part, as the world gives it: never reused.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct PartId(pub u64);

/// What a part is.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Shape {
    /// The unit box, in one paint.
    Cube,
    /// A model: a `.vox` file, named by the SHA-256 of its content, its
    /// voxels filling the unit box in their own palette (`vox`). The paint
    /// is nobody's. What a body meets of it is its own grid, read through
    /// the pose by whoever holds the file; here it is its box.
    Vox([u8; 32]),
    /// A picture: the box's front, its `y = -0.5` side, drawn with an image
    /// and seen from both sides. Wide as the box is along `x`, tall as it is
    /// along `z`; a hand places it in the picture's own aspect.
    Image(Media),
}

impl Shape {
    /// The model a file's asset name, `asset://<sha256>.vox`, stands for.
    /// `None` for a name that is no model's.
    pub fn of_asset(name: &str) -> Option<Shape> {
        let hex = name.strip_prefix("asset://")?.strip_suffix(".vox")?;
        if hex.len() != 64 {
            return None;
        }
        let mut hash = [0; 32];
        for (byte, pair) in hash.iter_mut().zip(hex.as_bytes().chunks(2)) {
            *byte = u8::from_str_radix(core::str::from_utf8(pair).ok()?, 16).ok()?;
        }
        Some(Shape::Vox(hash))
    }

    /// The asset name of a model's file. `None` for a shape of the core's.
    pub fn asset(self) -> Option<String> {
        match self {
            Shape::Vox(hash) => {
                let hex: String = hash.iter().map(|byte| format!("{byte:02x}")).collect();
                Some(format!("asset://{hex}.vox"))
            }
            Shape::Cube | Shape::Image(_) => None,
        }
    }
}

/// The content id of a heavy file, the SHA-256 of its bytes: how a record
/// names it (DECISIONS 89).
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct ContentId(pub [u8; 32]);

impl ContentId {
    /// The id as it is written: lower case hex.
    pub fn hex(&self) -> String {
        self.0.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    /// An id read back from its hex. `None` for anything else.
    pub fn parse(hex: &str) -> Option<ContentId> {
        let hex = hex.as_bytes();
        if hex.len() != 64 {
            return None;
        }
        let mut bytes = [0; 32];
        for (byte, pair) in bytes.iter_mut().zip(hex.chunks(2)) {
            let digit = |c: u8| (c as char).to_digit(16);
            *byte = (digit(pair[0])? * 16 + digit(pair[1])?) as u8;
        }
        Some(ContentId(bytes))
    }
}

/// The file kinds a picture is kept in, by the extension the world names
/// them with.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ImageKind {
    Png,
    Jpg,
    Webp,
}

impl ImageKind {
    pub fn ext(self) -> &'static str {
        match self {
            ImageKind::Png => "png",
            ImageKind::Jpg => "jpg",
            ImageKind::Webp => "webp",
        }
    }

    pub fn from_ext(ext: &str) -> Option<ImageKind> {
        match ext {
            "png" => Some(ImageKind::Png),
            "jpg" => Some(ImageKind::Jpg),
            "webp" => Some(ImageKind::Webp),
            _ => None,
        }
    }
}

/// The level of detail of a picture: which of its three files is shown.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum PictureLevel {
    /// At most 128 texels on its longest side, for what is far.
    Thumb,
    /// At most 1024, for what is near.
    Low,
    /// The original, for what is in view and close.
    Full,
}

/// What a picture is at every distance, and nothing else: its size in
/// texels and the three files its owner's client made when it was placed
/// (ARCHITECTURE § Files). The thumb and the low version are WebP.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Media {
    pub width: u32,
    pub height: u32,
    pub thumb: ContentId,
    pub low: ContentId,
    pub full: ContentId,
    /// What the original is kept as.
    pub kind: ImageKind,
}

impl Media {
    /// Width over height.
    pub fn aspect(&self) -> f64 {
        f64::from(self.width.max(1)) / f64::from(self.height.max(1))
    }

    /// The content id of a level.
    pub fn id(&self, level: PictureLevel) -> ContentId {
        match level {
            PictureLevel::Thumb => self.thumb,
            PictureLevel::Low => self.low,
            PictureLevel::Full => self.full,
        }
    }

    /// The asset reference of a level, as a record names a file:
    /// `asset://<hash>.<ext>`.
    pub fn name(&self, level: PictureLevel) -> String {
        let ext = match level {
            PictureLevel::Full => self.kind.ext(),
            _ => ImageKind::Webp.ext(),
        };
        format!("asset://{}.{ext}", self.id(level).hex())
    }

    /// A media read back from three asset names and a size. `None` when a
    /// name is no `asset://<hash>.<ext>` of a picture.
    pub fn parse(width: u32, height: u32, thumb: &str, low: &str, full: &str) -> Option<Media> {
        let split = |name: &str| -> Option<(ContentId, ImageKind)> {
            let (hex, ext) = name.strip_prefix("asset://")?.split_once('.')?;
            Some((ContentId::parse(hex)?, ImageKind::from_ext(ext)?))
        };
        let (full, kind) = split(full)?;
        Some(Media {
            width,
            height,
            thumb: split(thumb)?.0,
            low: split(low)?.0,
            full,
            kind,
        })
    }
}

/// Where a part stands and how it is turned and stretched, in the flat
/// frame of its seat.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Pose {
    /// Where its anchor stands: blocks along the seat's `u` and `v`, and
    /// blocks over the datum.
    pub at: [f64; 3],
    /// How it is turned, in the frame of where it stands: `x` along `u`, `y`
    /// along `v`, `z` up.
    pub rotation: DQuat,
    /// How long its unit box is along each of its own axes, in blocks.
    pub scale: [f64; 3],
}

impl Pose {
    /// Standing upright at a point, as big as `scale` says.
    pub fn upright(at: [f64; 3], scale: [f64; 3]) -> Pose {
        Pose {
            at,
            rotation: DQuat::IDENTITY,
            scale,
        }
    }

    /// The flat frame from the part's own space, around the anchor: how its
    /// unit box is turned and stretched. The anchor itself is not in it, so
    /// a seat adds where the anchor is.
    pub fn local(&self) -> DAffine3 {
        DAffine3::from_rotation_translation(self.rotation, DVec3::ZERO)
            * DAffine3::from_scale(DVec3::from(self.scale))
    }

    /// The part's own space from the flat frame, with the anchor in it: what
    /// a point of the seat is in the unit box.
    pub fn inward(&self) -> DAffine3 {
        self.local().inverse() * DAffine3::from_translation(-DVec3::from(self.at))
    }

    /// Whether the pose is sound: finite, a unit rotation, every length
    /// positive. What the world refuses otherwise.
    pub fn is_sound(&self) -> bool {
        let finite = self.at.iter().chain(&self.scale).all(|n| n.is_finite());
        finite
            && self.rotation.is_finite()
            && self.rotation.is_normalized()
            && self
                .scale
                .iter()
                .all(|&n| (MIN_SCALE..=MAX_SCALE).contains(&n))
    }
}

/// The least a part is along any axis, in blocks: thin as a pane, never
/// nothing.
pub const MIN_SCALE: f64 = 0.01;
/// The most, in blocks: half a kilometre, a plot's worth eight times over.
pub const MAX_SCALE: f64 = 1024.0;

/// A part as it is kept and told.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Part {
    pub shape: Shape,
    pub pose: Pose,
    /// What it is made of: a paint of the palette, as a cell's (`voxel`).
    pub paint: u8,
    /// Whether a body stands on it and is stopped by it.
    pub solid: bool,
    /// How much it shines with its own colour: 0 lit by what falls on it, 1
    /// its own colour as a screen shows it, more a lamp, up to [`MAX_SHINE`].
    pub shine: f32,
    /// How far the light of a part that is a lamp goes, metres, up to
    /// [`MAX_REACH_M`]. 0 is by its size.
    pub reach_m: f32,
}

/// The most a part shines with its own colour.
pub const MAX_SHINE: f32 = 8.0;
/// The furthest a part's light goes.
pub const MAX_REACH_M: f32 = 256.0;

impl Part {
    /// Whether what is said of the part beside its pose is sound: a shine
    /// and a reach that are numbers within theirs.
    pub fn is_sound(&self) -> bool {
        self.pose.is_sound()
            && self.shine.is_finite()
            && (0.0..=MAX_SHINE).contains(&self.shine)
            && self.reach_m.is_finite()
            && (0.0..=MAX_REACH_M).contains(&self.reach_m)
    }
}

/// What holds up a body standing beside a part: the top of it under the
/// feet and the bottom of it over the head, in blocks over the datum.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Gap {
    pub floor: Option<f64>,
    pub ceiling: Option<f64>,
}

impl Part {
    /// How far along a line from `from` toward `toward`, unit, both in the
    /// flat frame, the line goes into this part. `None` when it misses, or
    /// starts inside.
    pub fn hit(&self, from: DVec3, toward: DVec3) -> Option<f64> {
        self.hit_side(from, toward).map(|(blocks, _)| blocks)
    }

    /// Where a line in the flat frame first meets the part's box, blocks
    /// along the line, and which way the side it meets faces in the part's
    /// own space: `+z` is its top. `None` past it or from inside it.
    pub fn hit_side(&self, from: DVec3, toward: DVec3) -> Option<(f64, DVec3)> {
        let inward = self.pose.inward();
        let (from, toward) = (
            inward.transform_point3(from),
            inward.transform_vector3(toward),
        );
        let (near, far, side) = slab(from, toward)?;
        (near > 0.0 && far >= near).then_some((near, side))
    }

    /// What a body standing in a column has under its feet and over its
    /// head of this part, when its step reaches `reach` blocks over the
    /// datum: as the cells answer it (`voxel::Volumes::gap`). The body is
    /// `half` blocks wide each way of `column`. `None` where the part is
    /// not solid, or not over the body at all.
    ///
    /// Solid at the height of the step is a wall, and the wall's top is the
    /// floor it offers, which a walker refuses when it is more than a step
    /// up. Air there means the part below is the floor and the part above
    /// is the roof.
    pub fn gap(&self, column: [f64; 2], half: f64, reach: f64) -> Option<Gap> {
        if !self.solid {
            return None;
        }
        let foot = DVec3::new(column[0], column[1], 0.0);
        let feet = [
            DVec3::ZERO,
            DVec3::new(-half, -half, 0.0),
            DVec3::new(half, -half, 0.0),
            DVec3::new(-half, half, 0.0),
            DVec3::new(half, half, 0.0),
        ];
        gap_along(&self.pose.inward(), foot, DVec3::Z, &feet, reach)
    }

    /// How far a corner of this part reaches from its anchor, in blocks:
    /// what is within this of the anchor may be over a body, nothing
    /// further is.
    pub fn reach(&self) -> f64 {
        let [sx, sy, sz] = self.pose.scale;
        DVec3::new(sx / 2.0, sy / 2.0, sz).length()
    }

    /// The flat frame from the part's own space, with the anchor in it:
    /// where each corner of its unit box stands on the seat.
    pub fn placed(&self) -> DAffine3 {
        DAffine3::from_translation(DVec3::from(self.pose.at)) * self.pose.local()
    }
}

/// What a body standing at `foot` has under its feet and over its head of
/// a unit box, along `up`, when its step reaches `reach` along it: the
/// body's footprint is `foot` and each of `feet` added to it. `inward` is
/// the box's own space from the frame `foot`, `up` and `feet` are said in,
/// and the gap is in that frame's units along `up` from `foot`. `None`
/// where the box is not over the body at all.
///
/// Solid at the height of the step is a wall, and the wall's top is the
/// floor it offers, which a walker refuses when it is more than a step up.
/// Air there means the box below is the floor and the box above is the
/// roof.
pub fn gap_along(
    inward: &DAffine3,
    foot: DVec3,
    up: DVec3,
    feet: &[DVec3],
    reach: f64,
) -> Option<Gap> {
    let up_in = inward.transform_vector3(up);
    let mut gap: Option<Gap> = None;
    for &at in feet {
        let foot_in = inward.transform_point3(foot + at);
        let Some((low, high, _)) = slab(foot_in, up_in) else {
            continue;
        };
        let found = if (low <= reach && reach <= high) || high <= reach {
            Gap {
                floor: Some(high),
                ceiling: None,
            }
        } else {
            Gap {
                floor: None,
                ceiling: Some(low),
            }
        };
        gap = Some(match gap {
            None => found,
            Some(gap) => Gap {
                floor: pick(gap.floor, found.floor, f64::max),
                ceiling: pick(gap.ceiling, found.ceiling, f64::min),
            },
        });
    }
    gap
}

fn pick(a: Option<f64>, b: Option<f64>, f: fn(f64, f64) -> f64) -> Option<f64> {
    match (a, b) {
        (Some(a), Some(b)) => Some(f(a, b)),
        (a, b) => a.or(b),
    }
}

/// Where a line `from + toward * t`, in a part's own space, is inside the
/// unit box: the `t` it goes in and the `t` it comes out, or `None` when it
/// never is. A line along a side counts as inside it. What a shape with a
/// grid of its own walks between, and what a handle drawn as a box is
/// picked by.
pub fn through(from: DVec3, toward: DVec3) -> Option<(f64, f64)> {
    slab(from, toward).map(|(near, far, _)| (near, far))
}

/// Where a line enters and leaves the unit box, and which way the side it
/// enters by faces.
fn slab(from: DVec3, toward: DVec3) -> Option<(f64, f64, DVec3)> {
    let (low, high) = (DVec3::new(-0.5, -0.5, 0.0), DVec3::new(0.5, 0.5, 1.0));
    let (mut near, mut far) = (f64::NEG_INFINITY, f64::INFINITY);
    let mut side = DVec3::Z;
    for axis in 0..3 {
        let (d, f) = (toward[axis], from[axis]);
        if d.abs() < 1e-12 {
            if f < low[axis] || f > high[axis] {
                return None;
            }
            continue;
        }
        let (a, b) = ((low[axis] - f) / d, (high[axis] - f) / d);
        if a.min(b) > near {
            near = a.min(b);
            let mut normal = DVec3::ZERO;
            // Entering by the low side, the normal points down the axis.
            normal[axis] = if a < b { -1.0 } else { 1.0 };
            side = normal;
        }
        far = far.min(a.max(b));
    }
    (near <= far).then_some((near, far, side))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_says_which_side_of_the_box_it_meets() {
        let (near, _, side) = slab(DVec3::new(0.0, 0.0, 3.0), -DVec3::Z).unwrap();
        assert_eq!((near, side), (2.0, DVec3::Z), "the top from above");
        let (_, _, side) = slab(DVec3::new(-3.0, 0.0, 0.5), DVec3::X).unwrap();
        assert_eq!(side, -DVec3::X, "the low x side from the left");
        let (_, _, side) = slab(DVec3::new(0.0, 2.0, 0.5), -DVec3::Y).unwrap();
        assert_eq!(side, DVec3::Y, "the high y side from beyond it");
    }

    #[test]
    fn a_media_names_its_files_and_reads_them_back() {
        let hex = "0123456789abcdef".repeat(4);
        let thumb = format!("asset://{hex}.webp");
        let low = format!("asset://{}.webp", "ff".repeat(32));
        let full = format!("asset://{}.jpg", "00".repeat(32));
        let media = Media::parse(300, 200, &thumb, &low, &full).unwrap();
        assert_eq!(media.name(PictureLevel::Thumb), thumb);
        assert_eq!(media.name(PictureLevel::Low), low);
        assert_eq!(media.name(PictureLevel::Full), full);
        assert_eq!(media.kind, ImageKind::Jpg);
        assert!((media.aspect() - 1.5).abs() < 1e-12);
        assert!(Media::parse(1, 1, &thumb, &low, "asset://abc.jpg").is_none());
        assert!(Media::parse(1, 1, &thumb, &low, &format!("asset://{hex}.gif")).is_none());
        assert!(ContentId::parse(&"zz".repeat(32)).is_none());
    }
    use core::f64::consts::FRAC_PI_4;
    use proptest::prelude::*;

    fn cube(pose: Pose) -> Part {
        Part {
            shape: Shape::Cube,
            pose,
            paint: 0,
            solid: true,
            shine: 0.0,
            reach_m: 0.0,
        }
    }

    #[test]
    fn a_model_is_named_by_its_file() {
        let name = format!("asset://{}.vox", "0a".repeat(32));
        let shape = Shape::of_asset(&name).unwrap();
        assert_eq!(shape, Shape::Vox([0x0a; 32]));
        assert_eq!(shape.asset().as_deref(), Some(name.as_str()));
        assert_eq!(Shape::Cube.asset(), None);
        for wrong in [
            "asset://0a0a.vox",
            &format!("asset://{}.png", "0a".repeat(32)),
            &format!("{}.vox", "0a".repeat(32)),
            &format!("asset://{}.vox", "zz".repeat(32)),
        ] {
            assert_eq!(Shape::of_asset(wrong), None, "{wrong}");
        }
    }

    #[test]
    fn a_line_meets_a_cube_where_its_side_is() {
        let part = cube(Pose::upright([10.0, 10.0, 4.0], [2.0, 2.0, 2.0]));
        // From the side, along x: the near side is a block before the anchor.
        let hit = part.hit(DVec3::new(0.0, 10.0, 5.0), DVec3::X).unwrap();
        assert!((hit - 9.0).abs() < 1e-9, "{hit}");
        // From above: the top is two blocks over the anchor.
        let hit = part.hit(DVec3::new(10.0, 10.0, 20.0), -DVec3::Z).unwrap();
        assert!((hit - 14.0).abs() < 1e-9, "{hit}");
        // Past it.
        assert!(part.hit(DVec3::new(0.0, 12.0, 5.0), DVec3::X).is_none());
        // From inside: nothing is met.
        assert!(part.hit(DVec3::new(10.0, 10.0, 5.0), DVec3::X).is_none());
    }

    #[test]
    fn a_body_stands_on_a_cube_and_under_one() {
        let part = cube(Pose::upright([10.0, 10.0, 4.0], [2.0, 2.0, 2.0]));
        // Feet at the top: the floor is the top.
        let gap = part.gap([10.0, 10.0], 0.6, 7.0).unwrap();
        assert_eq!(
            gap,
            Gap {
                floor: Some(6.0),
                ceiling: None
            }
        );
        // Feet on the ground beside it, a step reaching into it: a wall,
        // whose top is the floor it offers.
        let gap = part.gap([10.0, 10.0], 0.6, 5.0).unwrap();
        assert_eq!(gap.floor, Some(6.0));
        // Feet well under it: a roof.
        let gap = part.gap([10.0, 10.0], 0.6, 1.0).unwrap();
        assert_eq!(
            gap,
            Gap {
                floor: None,
                ceiling: Some(4.0)
            }
        );
        // Beside it, out of reach of the body's width: nothing.
        assert!(part.gap([13.0, 10.0], 0.6, 5.0).is_none());
        // Within the body's width: the edge holds it.
        assert!(part.gap([11.4, 10.0], 0.6, 7.0).is_some());
        // Not solid: nothing, however near.
        let open = Part {
            solid: false,
            ..part
        };
        assert!(open.gap([10.0, 10.0], 0.6, 7.0).is_none());
    }

    #[test]
    fn a_turned_cube_is_met_on_its_turned_side() {
        // Turned an eighth about z, a cube of 2 reaches sqrt(2) from its
        // anchor along x.
        let pose = Pose {
            at: [0.0, 0.0, 0.0],
            rotation: DQuat::from_rotation_z(FRAC_PI_4),
            scale: [2.0, 2.0, 2.0],
        };
        let hit = cube(pose)
            .hit(DVec3::new(-10.0, 0.0, 1.0), DVec3::X)
            .unwrap();
        assert!((hit - (10.0 - 2f64.sqrt())).abs() < 1e-9, "{hit}");
        // Turned onto its side about y, its top is now where its side was.
        let pose = Pose {
            at: [0.0, 0.0, 0.0],
            rotation: DQuat::from_rotation_y(core::f64::consts::FRAC_PI_2),
            scale: [2.0, 2.0, 4.0],
        };
        let gap = cube(pose).gap([0.0, 0.0], 0.0, 10.0).unwrap();
        assert!((gap.floor.unwrap() - 1.0).abs() < 1e-9, "{gap:?}");
    }

    proptest! {
        #[test]
        fn what_a_line_meets_is_on_the_box(
            at in prop::array::uniform3(-50.0f64..50.0),
            axis in prop::array::uniform3(-1.0f64..1.0),
            angle in 0.0f64..6.3,
            scale in prop::array::uniform3(0.1f64..10.0),
            from in prop::array::uniform3(-100.0f64..100.0),
        ) {
            let axis = DVec3::from(axis);
            prop_assume!(axis.length() > 1e-3);
            let pose = Pose { at, rotation: DQuat::from_axis_angle(axis.normalize(), angle), scale };
            let part = cube(pose);
            let from = DVec3::from(from);
            let toward = (DVec3::from(at) - from).normalize();
            // Aimed at the anchor from outside, the line meets the box, and
            // where it does is on a side of the unit box.
            let hit = part.hit(from, toward);
            prop_assume!(hit.is_some());
            let on = pose.inward().transform_point3(from + toward * hit.unwrap());
            let side = [on.x.abs() - 0.5, on.y.abs() - 0.5, (on.z - 0.5).abs() - 0.5];
            prop_assert!(side.iter().any(|d| d.abs() < 1e-6), "{on:?}");
            prop_assert!(side.iter().all(|d| *d < 1e-6), "{on:?}");
        }
    }
}
