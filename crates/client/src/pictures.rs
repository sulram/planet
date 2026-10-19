//! Pictures in reach, at the level of detail each is seen at.
//!
//! A picture is named at three levels, made by its owner's client when it was
//! placed and kept as three files (ARCHITECTURE § Files): a thumb, a low
//! version and the original. Which one a quad shows is picked here, from how
//! tall the quad stands on the screen in texels: far draws the thumb, mid
//! the low version, near and in view the original. A level is asked for over
//! the asset seam when first wanted, as pixels the shell decoded, and dropped
//! when it goes unseen or the budget is full. Nothing comes down before its
//! replacement is up (DECISIONS 53): until the wanted level lands, the quad
//! shows the finest coarser level that has, or the card of a picture not yet
//! landed.

use std::collections::HashMap;

use glam::{DAffine3, DVec3};
use part::{ContentId, Media, PictureLevel as Level};
use scene::{Camera, Image, PictureChange, PictureDraw, PictureId};

use crate::assets::Requests;

/// The longest side of a thumb and of a low version, texels, as the owner's
/// client makes them.
pub const THUMB_SIDE: u32 = 128;
pub const LOW_SIDE: u32 = 1024;

/// The longest side a picture is decoded at: what a screen shows of a quad
/// two metres away. The original stays whole in its file; a texture wider
/// than this costs memory no view pays back.
pub const FULL_SIDE: u32 = 2048;

/// How many bytes of pictures stay resident, with their mip chains: a
/// hundred thumbs, a dozen low versions and a few originals at once.
pub const RESIDENT_BYTES: usize = 96 << 20;

/// How many levels may be in flight at once: a gallery walked into asks for
/// the nearest first, not for everything.
const IN_FLIGHT: usize = 4;

/// Frames a level stays resident unseen before it is dropped whatever the
/// budget: ten seconds at sixty.
const UNSEEN_FRAMES: u64 = 600;

/// A step to a finer level is taken when the quad has outgrown the level it
/// shows by this much, and a step to a coarser one when the coarser level
/// would still have texels to spare: the two edges never meet, so a quad
/// standing still never thrashes.
const OUTGROWN: f64 = 1.15;
const SPARE: f64 = 0.85;

/// The longest side a level is decoded at.
fn side(level: Level) -> u32 {
    match level {
        Level::Thumb => THUMB_SIDE,
        Level::Low => LOW_SIDE,
        Level::Full => FULL_SIDE,
    }
}

/// How many texels tall a level of a picture is, as the file holds it:
/// drawn down to its side, never grown.
fn height_texels(media: &Media, level: Level) -> f64 {
    let longest = f64::from(media.width.max(media.height).max(1));
    let height = f64::from(media.height.max(1));
    height * (f64::from(side(level)) / longest).min(1.0)
}

const LEVELS: [Level; 3] = [Level::Thumb, Level::Low, Level::Full];

fn coarser(level: Level) -> Option<Level> {
    match level {
        Level::Thumb => None,
        Level::Low => Some(Level::Thumb),
        Level::Full => Some(Level::Low),
    }
}

/// How a view sees: what turns a quad's height in metres into texels on the
/// screen, and what is in front of the eye.
#[derive(Clone, Copy, Debug)]
pub struct Screen {
    pub camera: Camera,
    /// Width over height.
    pub aspect: f64,
    /// The view's height in pixels.
    pub height_px: f64,
}

impl Screen {
    /// How tall a quad `height_m` tall at `centre` stands on the screen, in
    /// pixels, and whether any of it is in front of the eye: a quad behind
    /// the eye or off the sides is out of view, one in reach of the edge in.
    fn measure(&self, centre: DVec3, radius_m: f64, height_m: f64) -> (f64, bool) {
        let local = self.camera.rotation.inverse() * (centre - self.camera.position);
        let depth = -local.z;
        let tan = (f64::from(self.camera.fov_y) / 2.0).tan();
        let focal_px = self.height_px / (2.0 * tan);
        let apart = local.length().max(radius_m);
        let px = height_m / apart * focal_px;
        let in_view = depth + radius_m > 0.0
            && local.x.abs() <= (depth + radius_m) * tan * self.aspect + radius_m
            && local.y.abs() <= (depth + radius_m) * tan + radius_m;
        (px, in_view)
    }
}

struct Resident {
    id: PictureId,
    bytes: usize,
    /// The frame it was last wanted or shown in.
    seen: u64,
}

/// One picture as it is being shown: the level it last showed, so a step is
/// a step and not a flicker.
#[derive(Default)]
struct Shown {
    level: Option<Level>,
    seen: u64,
}

/// The pictures this client holds, by the content id of each level.
#[derive(Default)]
pub struct Pictures {
    resident: HashMap<ContentId, Resident>,
    /// Asked for and not yet answered, in the order asked.
    loading: Vec<ContentId>,
    /// What each picture shows, by the id of its original.
    shown: HashMap<ContentId, Shown>,
    next_id: u64,
    frame: u64,
    resident_bytes: usize,
    changes: Vec<PictureChange>,
}

impl Pictures {
    /// The level a quad shows this frame, and the draw of it.
    /// `world_from_quad` places the unit quad, its size in metres in it. The
    /// level wanted is asked for when it is not resident.
    pub(crate) fn draw(
        &mut self,
        media: &Media,
        world_from_quad: DAffine3,
        shine: f32,
        both_sides: bool,
        screen: &Screen,
        requests: &mut Requests,
    ) -> PictureDraw {
        let transform = world_from_quad;
        let width_m = transform.matrix3.x_axis.length();
        let height_m = transform.matrix3.y_axis.length();
        let radius_m = 0.5 * libm::hypot(width_m, height_m);
        let (px, in_view) = screen.measure(transform.translation, radius_m, height_m);

        let frame = self.frame;
        let shown = self.shown.entry(media.full).or_default();
        shown.seen = frame;
        let showing = shown.level;
        let wanted = if in_view {
            pick(media, showing, px)
        } else {
            Level::Thumb
        };
        let (level, picture) = self.show(media, wanted, requests);
        if let Some(level) = level {
            self.shown
                .get_mut(&media.full)
                .expect("entered above")
                .level = Some(level);
        }
        PictureDraw {
            picture,
            transform,
            shine,
            both_sides,
        }
    }

    /// The level shown for what is wanted: the wanted level when resident,
    /// else the finest coarser one that is, else the coarsest finer one.
    /// Asks for the wanted level when it is not resident and may.
    fn show(
        &mut self,
        media: &Media,
        wanted: Level,
        requests: &mut Requests,
    ) -> (Option<Level>, Option<PictureId>) {
        let frame = self.frame;
        let mut found = None;
        let mut level = Some(wanted);
        while let Some(at) = level {
            if let Some(resident) = self.resident.get_mut(&media.id(at)) {
                resident.seen = frame;
                found = Some((at, resident.id));
                break;
            }
            level = coarser(at);
        }
        // Nothing coarser landed: a finer level that did is better than a card.
        if found.is_none() {
            for at in LEVELS.into_iter().rev().filter(|&at| at > wanted) {
                if let Some(resident) = self.resident.get_mut(&media.id(at)) {
                    resident.seen = frame;
                    found = Some((at, resident.id));
                    break;
                }
            }
        }
        if found.is_none_or(|(at, _)| at != wanted) {
            self.want(media, wanted, requests);
        }
        match found {
            Some((at, id)) => (Some(at), Some(id)),
            None => (None, None),
        }
    }

    /// Asks for a level nobody has asked for yet, while there is room in
    /// flight. A level wanted and not asked for is asked for next frame.
    fn want(&mut self, media: &Media, level: Level, requests: &mut Requests) {
        let id = media.id(level);
        if self.loading.len() >= IN_FLIGHT || self.loading.contains(&id) {
            return;
        }
        self.loading.push(id);
        requests.ask_picture(media.name(level), id, side(level));
    }

    /// The answer to a request: pixels, or why there are none. A level that
    /// failed is forgotten, so asking again asks again.
    pub fn arrived(&mut self, id: ContentId, image: Result<Image, String>) -> Result<(), String> {
        self.loading.retain(|loading| *loading != id);
        let image = image?;
        // Four bytes a texel, and a third again for the mip chain.
        let bytes = (image.width as usize * image.height as usize * 4) * 4 / 3;
        self.next_id += 1;
        let picture = PictureId(self.next_id);
        self.changes
            .push(PictureChange::Add(picture, std::sync::Arc::new(image)));
        self.resident_bytes += bytes;
        self.resident.insert(
            id,
            Resident {
                id: picture,
                bytes,
                seen: self.frame,
            },
        );
        Ok(())
    }

    /// Closes a frame: what went unseen too long is dropped, and the least
    /// recently seen beyond that while the budget is blown. Called once every
    /// frame after every draw.
    pub fn end_frame(&mut self) {
        let frame = self.frame;
        let mut unseen: Vec<(u64, ContentId)> = self
            .resident
            .iter()
            .filter(|(_, resident)| resident.seen != frame)
            .map(|(id, resident)| (resident.seen, *id))
            .collect();
        unseen.sort();
        for (seen, id) in unseen {
            let stale = frame.saturating_sub(seen) > UNSEEN_FRAMES;
            if !stale && self.resident_bytes <= RESIDENT_BYTES {
                break;
            }
            if let Some(resident) = self.resident.remove(&id) {
                self.resident_bytes -= resident.bytes;
                self.changes.push(PictureChange::Remove(resident.id));
            }
        }
        self.shown
            .retain(|_, shown| frame.saturating_sub(shown.seen) <= UNSEEN_FRAMES);
        self.frame += 1;
    }

    /// Drops every picture: the world they stood in is gone.
    pub fn clear(&mut self) {
        for (_, resident) in self.resident.drain() {
            self.changes.push(PictureChange::Remove(resident.id));
        }
        self.resident_bytes = 0;
        self.loading.clear();
        self.shown.clear();
    }

    pub fn drain_changes(&mut self) -> Vec<PictureChange> {
        core::mem::take(&mut self.changes)
    }
}

/// The level a quad `px` pixels tall wants, given the one it shows: the
/// coarsest with texels enough, stepped to only past the edges.
fn pick(media: &Media, shown: Option<Level>, px: f64) -> Level {
    let enough = LEVELS
        .into_iter()
        .find(|&level| height_texels(media, level) >= px)
        .unwrap_or(Level::Full);
    let Some(current) = shown else {
        return enough;
    };
    if enough > current {
        // Finer wanted: only once the quad has clearly outgrown what it shows.
        if px > height_texels(media, current) * OUTGROWN {
            enough
        } else {
            current
        }
    } else if enough < current {
        // Coarser would do: only once it would do with room to spare.
        if px < height_texels(media, enough) * SPARE {
            enough
        } else {
            current
        }
    } else {
        current
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::{DQuat, DVec3};
    use part::ImageKind;

    fn id(byte: u8) -> ContentId {
        ContentId([byte; 32])
    }

    fn media() -> Media {
        Media {
            width: 3000,
            height: 2000,
            thumb: id(1),
            low: id(2),
            full: id(3),
            kind: ImageKind::Jpg,
        }
    }

    fn screen(height_px: f64) -> Screen {
        Screen {
            camera: Camera {
                position: DVec3::ZERO,
                rotation: DQuat::IDENTITY,
                fov_y: core::f32::consts::FRAC_PI_2,
                near: 0.1,
            },
            aspect: 16.0 / 9.0,
            height_px,
        }
    }

    /// A quad a metre tall in the picture's aspect, `apart` metres in front
    /// of the eye.
    fn ahead(apart: f64) -> DAffine3 {
        DAffine3::from_translation(DVec3::new(0.0, 0.0, -apart))
            * DAffine3::from_scale(DVec3::new(1.5, 1.0, 1.0))
    }

    fn pixels(width: u32, height: u32) -> Image {
        Image {
            width,
            height,
            rgba: vec![0; (width * height * 4) as usize],
        }
    }

    #[test]
    fn levels_are_as_tall_as_their_files() {
        let m = media();
        assert!((height_texels(&m, Level::Thumb) - 128.0 * 2.0 / 3.0).abs() < 1e-9);
        assert!((height_texels(&m, Level::Low) - 1024.0 * 2.0 / 3.0).abs() < 1e-9);
        assert!((height_texels(&m, Level::Full) - 2048.0 * 2.0 / 3.0).abs() < 1e-9);
        let small = Media {
            width: 64,
            height: 32,
            ..media()
        };
        assert_eq!(
            height_texels(&small, Level::Full),
            32.0,
            "a small picture is never upsized"
        );
    }

    #[test]
    fn far_wants_the_thumb_and_near_the_original() {
        let m = media();
        assert_eq!(pick(&m, None, 50.0), Level::Thumb);
        assert_eq!(pick(&m, None, 400.0), Level::Low);
        assert_eq!(pick(&m, None, 900.0), Level::Full);
        assert_eq!(pick(&m, None, 5000.0), Level::Full);
    }

    #[test]
    fn a_step_waits_past_the_edge() {
        let m = media();
        // Low is 682 texels tall. Just past it stays; well past it steps up.
        assert_eq!(pick(&m, Some(Level::Low), 700.0), Level::Low);
        assert_eq!(pick(&m, Some(Level::Low), 800.0), Level::Full);
        // Back down only when the thumb (85 texels) has room to spare.
        assert_eq!(pick(&m, Some(Level::Low), 80.0), Level::Low);
        assert_eq!(pick(&m, Some(Level::Low), 60.0), Level::Thumb);
    }

    #[test]
    fn a_quad_is_measured_on_the_screen() {
        let s = screen(1000.0);
        // A quad a metre tall at a right angle of view fills the height at
        // half a metre: a metre away it is half the screen.
        let (px, in_view) = s.measure(DVec3::new(0.0, 0.0, -1.0), 0.5, 1.0);
        assert!((px - 500.0).abs() < 1e-3, "{px}");
        assert!(in_view);
        let (_, behind) = s.measure(DVec3::new(0.0, 0.0, 2.0), 0.5, 1.0);
        assert!(!behind);
        let (_, aside) = s.measure(DVec3::new(10.0, 0.0, -1.0), 0.5, 1.0);
        assert!(!aside);
    }

    #[test]
    fn nothing_lands_before_it_is_asked_and_a_card_shows_meanwhile() {
        let mut pictures = Pictures::default();
        let mut requests = Requests::default();
        let m = media();
        let draw = pictures.draw(&m, ahead(2.0), 0.0, false, &screen(1000.0), &mut requests);
        assert_eq!(draw.picture, None, "a card until a level lands");
        let asked = requests.drain();
        assert_eq!(asked.len(), 1);
        assert_eq!(
            asked[0].path,
            m.name(Level::Low),
            "two metres away a metre tall is 250 px: the low version"
        );
        assert_eq!(
            asked[0].form,
            crate::assets::Form::Picture { max_side: LOW_SIDE }
        );
    }

    #[test]
    fn the_finest_coarser_level_shows_until_the_wanted_lands() {
        let mut pictures = Pictures::default();
        let mut requests = Requests::default();
        let m = media();
        pictures.arrived(m.thumb, Ok(pixels(128, 85))).unwrap();
        let near = ahead(1.0);
        let draw = pictures.draw(&m, near, 0.0, false, &screen(1000.0), &mut requests);
        let thumb = pictures.resident[&m.thumb].id;
        assert_eq!(draw.picture, Some(thumb), "the thumb stands in");
        let asked = requests.drain();
        assert_eq!(
            asked.iter().map(|r| r.path.clone()).collect::<Vec<_>>(),
            [m.name(Level::Low)]
        );
        pictures.arrived(m.low, Ok(pixels(1024, 682))).unwrap();
        let draw = pictures.draw(&m, near, 0.0, false, &screen(1000.0), &mut requests);
        assert_eq!(draw.picture, Some(pictures.resident[&m.low].id));
        assert!(
            requests.drain().is_empty(),
            "the wanted level is resident: nothing more asked"
        );
    }

    #[test]
    fn out_of_view_wants_the_thumb_alone() {
        let mut pictures = Pictures::default();
        let mut requests = Requests::default();
        let m = media();
        let behind = DAffine3::from_translation(DVec3::new(0.0, 0.0, 5.0));
        pictures.draw(&m, behind, 0.0, false, &screen(1000.0), &mut requests);
        let asked = requests.drain();
        assert_eq!(
            asked.iter().map(|r| r.path.clone()).collect::<Vec<_>>(),
            [m.name(Level::Thumb)]
        );
    }

    #[test]
    fn unseen_levels_are_dropped_and_the_budget_holds() {
        let mut pictures = Pictures::default();
        pictures.arrived(id(7), Ok(pixels(4, 4))).unwrap();
        let a = pictures.resident[&id(7)].id;
        for _ in 0..=UNSEEN_FRAMES {
            pictures.end_frame();
        }
        assert!(pictures.resident.contains_key(&id(7)), "kept to the edge");
        pictures.end_frame();
        assert!(!pictures.resident.contains_key(&id(7)));
        assert!(matches!(
            pictures.drain_changes().as_slice(),
            [PictureChange::Add(x, _), PictureChange::Remove(y)] if x == y && *x == a
        ));

        // Over budget, the least recently seen unseen level goes first.
        let big = (RESIDENT_BYTES as f64 / 4.0 / 4.0 * 3.0).sqrt() as u32 + 1;
        pictures.arrived(id(8), Ok(pixels(big, big))).unwrap();
        pictures.end_frame();
        pictures.arrived(id(9), Ok(pixels(big, big))).unwrap();
        pictures.end_frame();
        assert!(!pictures.resident.contains_key(&id(8)));
        assert!(
            pictures.resident.contains_key(&id(9)),
            "what was seen this frame stays"
        );
    }

    #[test]
    fn at_most_a_few_levels_fly_at_once() {
        let mut pictures = Pictures::default();
        let mut requests = Requests::default();
        for i in 0..10u8 {
            let m = Media {
                thumb: id(10 + i),
                low: id(30 + i),
                full: id(50 + i),
                ..media()
            };
            pictures.draw(&m, ahead(100.0), 0.0, false, &screen(1000.0), &mut requests);
        }
        assert_eq!(requests.drain().len(), IN_FLIGHT);
    }
}
