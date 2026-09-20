//! What the client costs in WASM, the slowest place it runs and the one where
//! the generator shares a thread with the frame. `scripts/bench.ts` times the
//! exports against budgets (CLAUDE.md, Performance).

use std::cell::RefCell;

use client::{Client, Input, Key, Recipe};
use wasm_bindgen::prelude::wasm_bindgen;
use worldgen::Generator;

thread_local! {
    static DESCENT: RefCell<Option<(Client, Input)>> = const { RefCell::new(None) };
}

/// Body numbers shared with the host.
const MOON: i32 = 1;

/// Samples `count` directions of a body at the footprint of its finest mesh.
/// A patch is 35 x 35 of them.
#[wasm_bindgen]
pub fn samples(body: i32, count: i32) -> f64 {
    let generator = Generator::new(Recipe::new(1)).expect("the current generator version");
    let mut sum = 0.0;
    for i in 0..count {
        let a = f64::from(i) * 1e-4;
        let direction = [a.cos(), a.sin() * 0.8, a.sin() * 0.6];
        let sample = if body == MOON {
            generator.moon_sample_at(direction, 0.8)
        } else {
            generator.sample_at(direction, 1.0)
        };
        sum += sample.height_m;
    }
    sum
}

/// Starts a descent from 3 km over a body to its ground: the streamer's
/// hardest moment, every level of the quadtree arriving at once.
#[wasm_bindgen]
pub fn descent_start(body: i32) {
    let mut client = Client::new(Recipe::new(1)).expect("the current generator version");
    if body == MOON {
        client.visit_moon(3000.0, -1.2, 6.0);
    } else {
        client.pose(3000.0, -1.2, 6.0);
    }
    let mut input = Input::default();
    input.key(Key::Down, true);
    DESCENT.with(|descent| *descent.borrow_mut() = Some((client, input)));
}

/// One frame of the descent. The host times the call.
#[wasm_bindgen]
pub fn descent_frame() {
    DESCENT.with(|descent| {
        let mut descent = descent.borrow_mut();
        let (client, input) = descent.as_mut().expect("descent_start comes first");
        client.update(1.0 / 60.0, input);
        client.drain_events();
        client.drain_terrain_changes();
    });
}
