//! The moon has a gravity field of its own: fly in, land, walk, jump high.

use client::{Client, Command, Input, Key, Mode, Recipe};

fn altitude_after(client: &mut Client, input: &mut Input, seconds: f64) -> (f64, f64) {
    let (mut last, mut peak) = (0.0, f64::MIN);
    for _ in 0..(seconds * 60.0) as u32 {
        client.update(1.0 / 60.0, input);
        for event in client.drain_events() {
            if let client::Event::Stats { altitude_m, .. } = event {
                last = altitude_m;
                peak = peak.max(altitude_m);
            }
        }
    }
    (last, peak)
}

fn jump_height(client: &mut Client) -> f64 {
    let mut input = Input::default();
    altitude_after(client, &mut input, 1.0);
    input.key(Key::Up, true);
    altitude_after(client, &mut input, 0.1);
    input.key(Key::Up, false);
    altitude_after(client, &mut input, 8.0).1
}

#[test]
fn flight_off_near_the_moon_means_landing_on_it() {
    let mut client = Client::new(Recipe::new(1)).unwrap();
    let on_planet = jump_height(&mut client);

    // A hundred metres under the moon, then flight off: the moon is down.
    client.visit_moon(100.0, 0.0, 6.0);
    client.command(Command::SetMode { mode: Mode::Walk });
    let mut input = Input::default();
    let (landed, _) = altitude_after(&mut client, &mut input, 30.0);
    assert!(landed.abs() < 0.01, "standing on the moon at {landed}");

    // Same legs, a fifth of the gravity.
    let on_moon = jump_height(&mut client);
    assert!(
        on_moon > on_planet * 3.0,
        "jumped {on_moon} against {on_planet} at home"
    );

    // Walking follows the craters, and a minute later you are still on the
    // ground: the moon carries you along its orbit.
    input.key(Key::Forward, true);
    let (walking, _) = altitude_after(&mut client, &mut input, 60.0);
    assert!(
        (-0.01..1.0).contains(&walking),
        "walking on the moon at {walking}"
    );

    // Flight wins over gravity: from the moon, fly away and the planet's frame
    // takes you back, with no gravity pulling the flyer round on the way.
    input.key(Key::Forward, false);
    client.command(Command::SetMode { mode: Mode::Fly });
    input.key(Key::Up, true);
    input.key(Key::Sprint, true);
    let (leaving, _) = altitude_after(&mut client, &mut input, 30.0);
    assert!(leaving > 30_000.0, "flew {leaving} m away from the moon");
}
