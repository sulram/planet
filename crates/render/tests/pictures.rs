//! A picture is drawn: a quad with its texture, lit, shining, and the card of
//! one not yet landed. Headless, to a PNG that is looked at (CLAUDE.md, How
//! it grows). Skipped where no GPU answers.

use std::sync::Arc;

use glam::{DAffine3, DQuat, DVec3, Vec3};
use render::{Headless, write_png};
use scene::{
    Camera, Effects, Frame, Image, InteractionCapsule, Moon, PictureChange, PictureDraw, PictureId,
};

/// A checkerboard with a red top left corner and a see-through band across
/// the middle, so the texture's way up and its cutout are both seen.
fn checker(width: u32, height: u32) -> Image {
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            let dark = ((x / 16) + (y / 16)) % 2 == 0;
            let mut texel = if dark {
                [40, 60, 200, 255]
            } else {
                [240, 240, 220, 255]
            };
            if x < 32 && y < 32 {
                texel = [220, 30, 30, 255];
            }
            if (height / 2 - 4..height / 2 + 4).contains(&y) {
                texel[3] = 0;
            }
            rgba.extend(texel);
        }
    }
    Image {
        width,
        height,
        rgba,
    }
}

#[test]
fn pictures_are_drawn() {
    let (width, height) = (640, 360);
    let Ok(mut headless) = Headless::new(width, height) else {
        eprintln!("no GPU: the picture test is skipped");
        return;
    };
    let id = PictureId(1);
    headless
        .renderer
        .apply_pictures(vec![PictureChange::Add(id, Arc::new(checker(256, 128)))]);

    // A small planet, the eye standing on it at its north pole, looking along -Z.
    let radius_m = 1000.0;
    let eye = DVec3::new(0.0, radius_m + 1.6, 0.0);
    let at = |x: f64, z: f64| DAffine3::from_translation(eye + DVec3::new(x, 0.0, z));
    let frame = Frame {
        camera: Camera {
            position: eye,
            rotation: DQuat::IDENTITY,
            fov_y: 60f32.to_radians(),
            near: 0.1,
        },
        sun_direction: Vec3::new(0.3, 0.8, 0.5).normalize(),
        moon: Moon {
            position: DVec3::new(0.0, 0.0, -50_000.0),
            radius_m: 100.0,
        },
        planet_radius_m: radius_m,
        clock_s: 0.0,
        patches: Vec::new(),
        shadow_patches: Vec::new(),
        interaction: InteractionCapsule {
            start: eye,
            end: eye,
            radius_m: 0.0,
        },
        effects: Effects {
            clouds: false,
            ..Effects::default()
        },
        boxes: Vec::new(),
        skinned: Vec::new(),
        volumes: Vec::new(),
        glass: Vec::new(),
        lamps: Vec::new(),
        models: Vec::new(),
        ghost: None,
        guides: Vec::new(),
        pictures: vec![
            // Lit, two to one, facing the eye.
            PictureDraw {
                picture: Some(id),
                transform: at(0.0, -3.0) * DAffine3::from_scale(DVec3::new(2.0, 1.0, 1.0)),
                shine: 0.0,
                both_sides: false,
            },
            // A screen, turned away and seen from behind: drawn, left still left.
            PictureDraw {
                picture: Some(id),
                transform: at(-2.6, -3.0)
                    * DAffine3::from_quat(DQuat::from_rotation_y(core::f64::consts::PI))
                    * DAffine3::from_scale(DVec3::new(2.0, 1.0, 1.0)),
                shine: 1.0,
                both_sides: true,
            },
            // A poster seen from behind: not drawn.
            PictureDraw {
                picture: Some(id),
                transform: at(-2.6, -1.5)
                    * DAffine3::from_quat(DQuat::from_rotation_y(core::f64::consts::PI))
                    * DAffine3::from_scale(DVec3::new(0.6, 0.3, 1.0)),
                shine: 0.0,
                both_sides: false,
            },
            // The card of a picture not yet landed.
            PictureDraw {
                picture: None,
                transform: at(2.4, -3.0) * DAffine3::from_scale(DVec3::new(1.5, 1.0, 1.0)),
                shine: 0.0,
                both_sides: false,
            },
        ],
    };
    let pixels = headless.render(&frame);
    // Beside the other test output, to be looked at: `target/tmp/pictures.png`.
    let out = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("pictures.png");
    write_png(&out, width, height, &pixels).unwrap();

    let texel = |x: u32, y: u32| {
        let i = ((y * width + x) * 4) as usize;
        [pixels[i], pixels[i + 1], pixels[i + 2]]
    };
    // The quad in the middle is a metre tall three metres away under a 60
    // degree view: 104 pixels tall, 208 wide, about the centre.
    let centre = texel(320, 180);
    let sky = texel(320, 20);
    assert_ne!(centre, sky, "the picture covers the centre");
    let corner = texel(320 - 100, 180 - 48);
    assert!(
        corner[0] > corner[2] + 60,
        "the red corner is the top left of the quad: {corner:?}"
    );
    let band = texel(320 + 40, 180);
    let above = texel(320 + 40, 180 - 20);
    assert_ne!(band, above, "the see-through band shows what is behind");
}
