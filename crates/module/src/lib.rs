//! The server's module (DECISIONS 97, 99, 102): every world half of a
//! version, hosted by `world::Host`, built as one WASM file the Go server
//! runs through wazero.
//!
//! The server asks by [`serve`]: one call in, every reply out, as bytes in
//! the schema of `proto/planet/module/v1`. In the WASM build `abi` gives that
//! one function the three names the server knows; natively a test calls it
//! as it is.

use std::sync::Mutex;

use world::{Host, Measure};

#[cfg(target_family = "wasm")]
mod abi;

/// The host, made on the first call with the plugins the version carries.
static HOST: Mutex<Option<Host>> = Mutex::new(None);

/// One call of the server, and every reply to it, each encoded whole.
pub fn serve(call: &[u8]) -> Vec<Vec<u8>> {
    HOST.lock()
        .expect("the server calls one call at a time")
        .get_or_insert_with(|| Host::new(plugins_world::all(), measure_of))
        .serve(call)
}

/// The size of the bodies a recipe makes. The recipe on the wire carries no
/// size of its own (OPEN.md), so a hosted world has the generator's, whatever
/// else its recipe says: a seed or params the generator cannot read found a
/// world no client draws, and its plugins are hosted all the same.
fn measure_of(_recipe: &protocol::Recipe) -> Option<Measure> {
    Some(Measure {
        sphere: worldgen::Recipe::new(0).sphere()?,
        moon_radius_m: worldgen::MOON_RADIUS_M,
    })
}
