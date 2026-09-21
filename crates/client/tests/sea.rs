//! The sea is penetrable: you wade in, float, dive and touch the floor.

use client::{Client, Input, Key, Recipe};

/// A point of sector 0 well under water, found by scanning the generator.
fn open_sea() -> [f64; 2] {
    let generator = worldgen::Generator::new(Recipe::new(1)).unwrap();
    let sphere = generator.sphere();
    let side = f64::from(sphere.blocks().side());
    for i in 1..40 {
        for j in 1..40 {
            let (u, v) = (f64::from(i) / 40.0, f64::from(j) / 40.0);
            let point = topology::SurfacePoint::new(topology::Sector::ALL[0], u * side, v * side);
            if generator.sample(sphere.blocks().direction(point)).height_m < -40.0 {
                return [u, v];
            }
        }
    }
    panic!("seed 1 has no sea in sector 0");
}

fn run(client: &mut Client, input: &mut Input, seconds: f64) -> f64 {
    let mut altitude = 0.0;
    for _ in 0..(seconds * 60.0) as u32 {
        client.update(1.0 / 60.0, input);
        for event in client.drain_events() {
            if let client::Event::Stats { altitude_m, .. } = event {
                altitude = altitude_m;
            }
        }
    }
    altitude
}

#[test]
fn a_swimmer_floats_dives_and_surfaces() {
    let [u, v] = open_sea();
    println!("open sea at {u},{v}");
    let mut client = Client::new(Recipe::new(1)).unwrap();
    client.teleport(u, v);
    client.pose(-20.0, 0.0, 6.0); // released twenty metres down
    let mut input = Input::default();

    // Doing nothing, a body drifts up to floating depth and stays there.
    let floating = run(&mut client, &mut input, 40.0);
    assert!((-0.95..-0.65).contains(&floating), "floating at {floating}");

    // Holding down dives; the sea floor is the limit, far below.
    input.key(Key::Down, true);
    let diving = run(&mut client, &mut input, 6.0);
    assert!(diving < -8.0, "dived to {diving}");

    // Holding up comes back to the surface, where it turns into leaps: never
    // deeper than floating depth, never higher than a jump.
    input.key(Key::Down, false);
    input.key(Key::Up, true);
    let surfaced = run(&mut client, &mut input, 20.0);
    assert!((-0.95..2.5).contains(&surfaced), "surfaced at {surfaced}");

    // Let go: back to floating.
    input.key(Key::Up, false);
    let resting = run(&mut client, &mut input, 6.0);
    assert!((-0.95..-0.65).contains(&resting), "resting at {resting}");

    // Looking well down and swimming forward is a dive too: no key needed.
    input.look = [0.0, 400.0];
    input.key(Key::Forward, true);
    let steered = run(&mut client, &mut input, 6.0);
    assert!(steered < -4.0, "steered down to {steered}");
}
