//! What the client costs in WASM, the slowest place it runs and the one where
//! the generator shares a thread with the frame. `scripts/bench.ts` times the
//! exports against budgets (CLAUDE.md, Performance).

use std::cell::RefCell;

use client::collision;
use client::{Client, Event, Input, Key, Recipe};
use topology::{Sector, SurfacePoint};
use wasm_bindgen::prelude::wasm_bindgen;
use worldgen::{Field, Generator, Source};

thread_local! {
    static DESCENT: RefCell<Option<(Client, Input)>> = const { RefCell::new(None) };
}

/// Body numbers shared with the host.
const MOON: i32 = 1;
/// The planet again, shaped by the field the host loaded.
const FIELD: i32 = 2;

thread_local! {
    static GROUND: RefCell<Option<Field>> = const { RefCell::new(None) };
}

/// Hands over a baked field, so the host can time a world shaped by one. The
/// host skips these when the field has not been baked.
#[wasm_bindgen]
pub fn load_field(bytes: Vec<u8>) {
    let field = Field::parse(bytes).expect("a baked field");
    GROUND.with(|held| *held.borrow_mut() = Some(field));
}

/// The recipe of a world over the loaded field, for the bodies that want one.
fn over_field(body: i32) -> Option<(Recipe, Field)> {
    if body != FIELD {
        return None;
    }
    GROUND.with(|held| {
        held.borrow().clone().map(|field| {
            let mut recipe = Recipe::new(1);
            recipe.params.source = Source::Field(field.id());
            (recipe, field)
        })
    })
}

/// Samples `count` directions of a body at the footprint of its finest mesh.
/// A patch is 35 x 35 of them.
#[wasm_bindgen]
pub fn samples(body: i32, count: i32) -> f64 {
    let generator = match over_field(body) {
        Some((recipe, field)) => Generator::with_field(recipe, field).expect("the loaded field"),
        None => Generator::new(Recipe::new(1)).expect("the current generator version"),
    };
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

/// One patch of volume: the columns of the finest quadtree patch, and a
/// window of cells up each of them. The heightfield patch it would replace is
/// `samples`, so the two numbers are what decide whether a volume can be
/// meshed inside a frame at all.
#[wasm_bindgen]
pub fn volume(patches: i32, columns: i32, cells: i32) -> f64 {
    let generator = Generator::new(Recipe::new(1)).expect("the current generator version");
    let mut sum = 0.0;
    for patch in 0..patches {
        for i in 0..columns {
            let a = f64::from(patch) * 1e-3 + f64::from(i) * 1e-5;
            let direction = [a.cos(), a.sin() * 0.8, a.sin() * 0.6];
            let column = generator.column(direction, 0.5);
            let ground_m = column.ground().height_m;
            for k in 0..cells {
                sum += column.density_m(ground_m - f64::from(k) * 0.5);
            }
        }
    }
    sum
}

thread_local! {
    static WALK: RefCell<Option<(Client, Input)>> = const { RefCell::new(None) };
}

/// The seed and the place the caves were found in (docs/ROADMAP.md).
const CAVE_SEED: u64 = 0x0000_0000_cafe_0007;
/// A mouth: walking forward from here goes under the ground inside five
/// seconds, so the walk below pays for the footings and for the patches the
/// body asks to be meshed deeper.
const CAVE_AT: [f64; 2] = [0.3105, 0.6585];

/// Footings across cave country: what one body costs the frame it walks in.
/// A step takes one where the body stands and up to three more for where it
/// might go, so this is multiplied by four, and again by every agent in a
/// world. The columns here are the expensive kind: a column with no cave in
/// it answers from its height and never probes.
#[wasm_bindgen]
pub fn footings(count: i32) -> f64 {
    let generator = Generator::new(Recipe::new(CAVE_SEED)).expect("the current generator version");
    let sphere = generator.sphere();
    let side = f64::from(sphere.blocks().side());
    let mut sum = 0.0;
    for i in 0..count {
        let spread = f64::from(i) * 1e-5;
        let point = SurfacePoint::new(
            Sector::ALL[0],
            (CAVE_AT[0] + spread) * side,
            (CAVE_AT[1] + spread) * side,
        );
        let direction = sphere.blocks().direction(point);
        let ground_m = generator.sample(direction).height_m;
        sum += collision::footing(&generator, direction, ground_m)
            .floor_m
            .unwrap_or_default();
    }
    sum
}

/// Stands a body in cave country and starts it running. This is the frame
/// that reads the density field under the feet: a footing where the body is,
/// and one for each step it tries before it takes one.
#[wasm_bindgen]
pub fn walk_start() {
    let mut client = Client::new(Recipe::new(CAVE_SEED)).expect("the current generator version");
    {
        let side = f64::from(client.sphere().blocks().side());
        client.teleport(topology::SurfacePoint::new(
            topology::Sector::ALL[0],
            CAVE_AT[0] * side,
            CAVE_AT[1] * side,
        ));
    }
    client.pose(0.0, -0.2, 6.0);
    let mut input = Input::default();
    input.key(Key::Forward, true);
    input.key(Key::Sprint, true);
    WALK.with(|walk| *walk.borrow_mut() = Some((client, input)));
}

/// One frame of the walk. The host times the call.
#[wasm_bindgen]
pub fn walk_frame() {
    WALK.with(|walk| {
        let mut walk = walk.borrow_mut();
        let (client, input) = walk.as_mut().expect("walk_start comes first");
        client.update(1.0 / 60.0, input);
        client.drain_events();
        client.drain_terrain_changes();
    });
}

/// Starts a descent from 3 km over a body to its ground: the streamer's
/// hardest moment, every level of the quadtree arriving at once.
#[wasm_bindgen]
pub fn descent_start(body: i32) {
    let mut client = match over_field(body) {
        Some((recipe, field)) => Client::with_field(recipe, field).expect("the loaded field"),
        None => Client::new(Recipe::new(1)).expect("the current generator version"),
    };
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

thread_local! {
    static LAY: RefCell<Option<Client>> = const { RefCell::new(None) };
}

/// The steepest plot found near the test world, on sector 4: 32 m of fall
/// across its 32 m, where a solid base is the most cells a platform lays.
const STEEP_AT: [f64; 2] = [0.312, 0.372];

/// Stands a body on the steepest plot and lets the ground settle there, so
/// what follows times the build alone.
#[wasm_bindgen]
pub fn lay_start() {
    let mut client = Client::new(Recipe::new(1)).expect("the current generator version");
    {
        let side = f64::from(client.sphere().blocks().side());
        client.teleport(SurfacePoint::new(
            Sector::ALL[4],
            STEEP_AT[0] * side,
            STEEP_AT[1] * side,
        ));
    }
    client.command_json(r#"{"type":"set_platform","side":64}"#);
    client.settled_frame();
    client.drain_events();
    client.drain_terrain_changes();
    client.drain_volume_changes();
    LAY.with(|lay| *lay.borrow_mut() = Some(client));
}

/// Lays a platform of 64 on a solid base, as a front end asks for it
/// between two frames. The host times the call.
#[wasm_bindgen]
pub fn lay_platform() {
    LAY.with(|lay| {
        let mut lay = lay.borrow_mut();
        let client = lay.as_mut().expect("lay_start comes first");
        client.command_json(r#"{"type":"lay_platform","base":"solid"}"#);
    });
}

/// One frame after it. True once the platform is laid and drawn whole,
/// which the client says as it settles.
#[wasm_bindgen]
pub fn lay_frame() -> bool {
    LAY.with(|lay| {
        let mut lay = lay.borrow_mut();
        let client = lay.as_mut().expect("lay_start comes first");
        client.update(1.0 / 60.0, &mut Input::default());
        client.drain_terrain_changes();
        client.drain_volume_changes();
        let events = client.drain_events();
        events.iter().any(|event| matches!(event, Event::Settled))
    })
}
