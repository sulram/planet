//! A vertical cut through the ground, as a PNG.
//!
//! The volume layer is the one thing a screenshot cannot show: a cave is
//! behind the ground, and the renderer draws the ground. So the density is
//! read straight and painted, one pixel a sample, which is how a cave, a
//! bedrock floor and a sea that should not be hollow are looked at rather
//! than believed (CLAUDE.md, How it grows).

use render::write_png;
use topology::{BLOCK_M, Sector, SurfacePoint};
use worldgen::Generator;

use crate::args::Shot;

/// Metres above and below the ground at the middle of the cut.
const SKY_M: f64 = 60.0;
const DEEP_M: f64 = 220.0;

pub fn run(shot: Shot, generator: &Generator) -> Result<(), String> {
    let [width, height] = shot.size;
    let span_m = shot.slice_m;
    let sector = Sector::new(0).expect("sector 0");
    let side = f64::from(topology::SECTOR_SIDE);
    let [u, v] = shot.at.unwrap_or([0.5, 0.5]);
    let (middle_u, v) = (u * side, v * side);

    // The ground under the middle of the cut sets where the window sits, so
    // the picture is about the surface wherever on the planet it is taken.
    let at = |u: f64| SurfacePoint::new(sector, u, v).direction();
    let centre_m = generator.sample(at(middle_u)).height_m;
    let (top_m, bottom_m) = (centre_m + SKY_M, centre_m - DEEP_M);

    let mut rgba = vec![0u8; (width * height * 4) as usize];
    for x in 0..width {
        let across = (f64::from(x) + 0.5) / f64::from(width) - 0.5;
        let direction = at(middle_u + across * span_m / BLOCK_M);
        let sample = generator.sample(direction);
        for y in 0..height {
            let down = (f64::from(y) + 0.5) / f64::from(height);
            let height_m = top_m + (bottom_m - top_m) * down;
            let density_m = generator.density_m(direction, height_m, 0.0);
            let colour = paint(density_m, height_m, sample.height_m);
            let at = ((y * width + x) * 4) as usize;
            rgba[at..at + 4].copy_from_slice(&colour);
        }
    }
    write_png(&shot.out, width, height, &rgba)?;
    log::info!(
        "{} m of ground, {:.0} m to {:.0} m -> {}",
        span_m,
        bottom_m,
        top_m,
        shot.out.display()
    );
    Ok(())
}

/// Solid darkens with depth, air is pale, sea is blue, and the skin of the
/// ground is drawn bright so a cave wall reads as a wall.
fn paint(density_m: f64, height_m: f64, ground_m: f64) -> [u8; 4] {
    let skin = density_m.abs() < 0.6;
    if density_m > 0.0 {
        if skin {
            return [255, 214, 120, 255];
        }
        let depth = (density_m / 180.0).clamp(0.0, 1.0);
        let shade = (120.0 - 70.0 * depth) as u8;
        [
            shade,
            (shade as f64 * 0.88) as u8,
            (shade as f64 * 0.76) as u8,
            255,
        ]
    } else if skin {
        [255, 214, 120, 255]
    } else if height_m < 0.0 && ground_m < 0.0 {
        [40, 90, 130, 255]
    } else if height_m < 0.0 {
        // A cave mouth below sea level, or a void the sea would fill.
        [70, 130, 170, 255]
    } else {
        [225, 235, 245, 255]
    }
}
