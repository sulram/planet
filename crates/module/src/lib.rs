//! The server's module (DECISIONS 97, 99, 102, 110): the systems of the core
//! that run in the world, the cells, and every world half of a version,
//! hosted by `world::Host`, built as one WASM file the Go server runs
//! through wazero.
//!
//! The server asks by [`serve`]: one call in, every reply out, as bytes in
//! the schema of `proto/planet/module/v1`. In the WASM build `abi` gives that
//! one function the names the server knows and asks the server for what an
//! owner keeps; natively a test calls it as it is, over a store held here.

use std::sync::Mutex;

use world::{Host, Measure};
use worldgen::{Generator, Params, Recipe};

#[cfg(target_family = "wasm")]
mod abi;
#[cfg(target_family = "wasm")]
use abi::asked;
#[cfg(not(target_family = "wasm"))]
mod alone;
#[cfg(not(target_family = "wasm"))]
use alone::asked;

static HOST: Mutex<Option<Host>> = Mutex::new(None);

/// Serves one call of the server and gives back what was said, each reply
/// encoded. The host is made by the first call.
pub fn serve(call: &[u8]) -> Vec<Vec<u8>> {
    let said = HOST
        .lock()
        .expect("the server calls one call at a time")
        .get_or_insert_with(|| {
            let core = vec![world::cells::system()];
            Host::new(core, plugins_world::all(), measure_of, ground_of)
        })
        .serve(call, &mut asked);
    #[cfg(not(target_family = "wasm"))]
    alone::keep(&said);
    said
}

/// How a recipe says the size of its bodies. The wire's recipe carries no
/// size of its own (OPEN), so every world a server hosts is measured at the
/// generator's default, whatever else its recipe says.
fn measure_of(_recipe: &protocol::Recipe) -> Option<Measure> {
    Some(Measure {
        sphere: Recipe::new(0).sphere()?,
        moon_radius_m: worldgen::MOON_RADIUS_M,
    })
}

/// The ground a recipe makes, for the systems that read it: where a volume
/// starts and ends (DECISIONS 107). `None` for a recipe this generator
/// cannot read, and for a world shaped by a field, which the server does
/// not hold (OPEN).
fn ground_of(recipe: &protocol::Recipe) -> Option<Generator> {
    let params = match recipe.params_json.trim() {
        "" => Params::default(),
        json => serde_json::from_str(json).ok()?,
    };
    let seed = worldgen::parse_seed(&recipe.seed).ok()?;
    Generator::new(Recipe {
        generator_version: recipe.generator_version,
        params,
        ..Recipe::new(seed)
    })
    .ok()
}
