//! Generator version 1. Frozen once a release ships it: fix nothing here that
//! changes output, write `v2` instead.
//!
//! Three layers of 3D noise sampled on the unit sphere (no seams, no
//! projection distortion): continents, mountain ridges on land, fine detail.

use crate::noise::{fbm, ridged, smoothstep};
use crate::{Direction, Material, Recipe, Sample};

// Each layer has its own seed stream.
const CONTINENT: u64 = 0x1000;
const WARP: u64 = 0x2000;
const MOUNTAIN: u64 = 0x3000;
const HILL: u64 = 0x4000;
const DETAIL: u64 = 0x5000;
const MOISTURE: u64 = 0x6000;

pub fn sample(recipe: &Recipe, d: Direction) -> Sample {
    let seed = recipe.seed;
    let params = &recipe.params;
    let at = |k: f64| [d[0] * k, d[1] * k, d[2] * k];

    // Continents: low frequency, bent by a warp so coasts are not blobs.
    let warp = fbm(seed ^ WARP, at(params.continent_scale * 2.0), 3, 0.5) * 0.35;
    let c = at(params.continent_scale);
    let continent = fbm(
        seed ^ CONTINENT,
        [c[0] + warp, c[1] - warp, c[2] + warp],
        6,
        0.5,
    );
    // Shift the zero crossing so about `sea_share` of the planet is under water.
    let land = continent - (params.sea_share - 0.5) * 0.9;

    let height_m = if land < 0.0 {
        // Shelf first, then the deep.
        -params.ocean_depth_m * smoothstep(0.0, 0.45, -land)
    } else {
        let inland = smoothstep(0.0, 0.18, land);
        let ranges = smoothstep(
            -0.1,
            0.5,
            fbm(seed ^ HILL, at(params.continent_scale * 3.0), 3, 0.5),
        );
        let peaks = ridged(seed ^ MOUNTAIN, at(params.continent_scale * 8.0), 7, 0.55);
        let hills = fbm(seed ^ HILL, at(params.continent_scale * 40.0), 5, 0.5);
        let detail = fbm(seed ^ DETAIL, at(2600.0), 4, 0.5);
        params.relief_m * inland * (0.04 + 0.10 * land + 0.86 * ranges * peaks * peaks)
            + inland * (18.0 * hills + 1.2 * detail)
            + 1.5 * land / (land + 0.02)
    };

    Sample {
        height_m,
        material: material(recipe, d, height_m),
        shade: 0.0,
    }
}

fn material(recipe: &Recipe, d: Direction, height_m: f64) -> Material {
    if height_m < 0.0 {
        return Material::Water;
    }
    let seed = recipe.seed;
    let relief = recipe.params.relief_m;
    // Bands wobble so they do not read as contour lines.
    let wobble = fbm(
        seed ^ DETAIL,
        [d[0] * 90.0, d[1] * 90.0, d[2] * 90.0],
        3,
        0.5,
    );
    let h = height_m + wobble * 0.06 * relief;
    // Cold near the poles (the Y axis) and up high.
    let cold = libm::fabs(d[1]) + h / relief * 0.9;
    if cold > 0.93 {
        return Material::Snow;
    }
    if height_m < 2.5 {
        return Material::Sand;
    }
    if h > 0.42 * relief {
        return Material::Rock;
    }
    let moisture = fbm(
        seed ^ MOISTURE,
        [d[0] * 5.0, d[1] * 5.0, d[2] * 5.0],
        4,
        0.5,
    );
    if moisture > 0.05 {
        Material::Forest
    } else {
        Material::Grass
    }
}
