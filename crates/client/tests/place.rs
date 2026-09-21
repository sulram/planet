//! Saying where you are: the code a person reads out, and the way they face.

use client::{Client, Event, Input, Key, Recipe};

/// Everything the HUD shows about a place, as the client last said it.
struct Said {
    place: String,
    pose: String,
    bearing_deg: Option<f64>,
}

fn run(client: &mut Client, input: &mut Input, seconds: f64) -> Said {
    let mut said = None;
    for _ in 0..(seconds * 60.0) as u32 {
        client.update(1.0 / 60.0, input);
        for event in client.drain_events() {
            if let Event::Stats {
                place,
                pose,
                bearing_deg,
                ..
            } = event
            {
                said = Some(Said {
                    place,
                    pose,
                    bearing_deg,
                });
            }
        }
    }
    said.expect("the client says where it is")
}

#[test]
fn a_place_is_a_sector_and_seven_characters() {
    let mut client = Client::new(Recipe::new(1)).expect("a world");
    let said = run(&mut client, &mut Input::default(), 2.0);

    let (sector, code) = said.place.split_once('-').expect("a dash after the sector");
    assert_eq!(sector.len(), 1, "one sector digit in {}", said.place);
    assert!(
        ('0'..'6').contains(&sector.chars().next().expect("a digit")),
        "sector out of range in {}",
        said.place
    );
    assert_eq!(code.len(), topology::CODE_MAX, "full precision by default");
    assert!(
        code.chars()
            .all(|c| "0123456789ABCDEFGHJKMNPQRSTVWXYZ".contains(c)),
        "{code} is not in the alphabet"
    );
}

#[test]
fn walking_changes_the_tail_and_keeps_the_head() {
    let mut client = Client::new(Recipe::new(1)).expect("a world");
    let mut input = Input::default();
    let before = run(&mut client, &mut input, 2.0);

    input.key(Key::Forward, true);
    let after = run(&mut client, &mut input, 6.0);

    assert_ne!(
        before.place, after.place,
        "six seconds of walking moved nobody"
    );
    // Three characters of code are fifteen bits, eight of `u` and seven of
    // `v`, so they name a box of about 128 by 256 metres of the reference
    // body. A walk is metres. If the head moved, either the code is not
    // hierarchical or the avatar was teleported, and both are worth failing
    // over.
    assert_eq!(
        &before.place[..5],
        &after.place[..5],
        "a walk left the box its own code names: {} to {}",
        before.place,
        after.place
    );
}

#[test]
fn turning_around_turns_the_compass_around() {
    let mut client = Client::new(Recipe::new(1)).expect("a world");
    let mut input = Input::default();
    // Walk first: facing follows where a body goes, so a standing avatar has
    // nothing to report yet.
    input.key(Key::Forward, true);
    let before = run(&mut client, &mut input, 3.0)
        .bearing_deg
        .expect("a bearing");

    // Half a turn of the view, then long enough for the body to follow it.
    input.look = [1800.0, 0.0];
    run(&mut client, &mut input, 1.0);
    input.look = [0.0, 0.0];
    let after = run(&mut client, &mut input, 3.0)
        .bearing_deg
        .expect("a bearing");

    let turned = (after - before + 360.0) % 360.0;
    assert!(
        (90.0..270.0).contains(&turned),
        "half a turn read as {turned} degrees, from {before} to {after}"
    );
}

/// The compass is only right if it agrees with the sky. A swapped east reads
/// as a plausible number and shows up nowhere else, so this asks the one
/// question that cannot be argued with.
///
/// Not "the bearing of the sun rises": that is true north of the tropics and
/// false south of them, where the sun crosses through north and the bearing
/// falls. What holds for every observer on every body is that the sun *moves*
/// west, so it is the motion that is asked about and not the position.
#[test]
fn the_sun_moves_west_wherever_you_stand() {
    let mut client = Client::new(Recipe::new(1)).expect("a world");
    let mut input = Input::default();

    let mut sun_at = |client: &mut Client, seconds: f64| {
        client.set_clock(seconds);
        let frame = client.update(1.0 / 60.0, &mut input);
        client.drain_events();
        let s = frame.sun_direction;
        [f64::from(s.x), f64::from(s.y), f64::from(s.z)]
    };
    // Two moments of one day, which is 1200 seconds.
    let before = sun_at(&mut client, 0.0);
    let after = sun_at(&mut client, 60.0);
    let motion = [0, 1, 2].map(|i| after[i] - before[i]);

    let bearing = topology::bearing_deg(client.up(), motion).expect("the sun goes somewhere");
    assert!(
        (180.0..360.0).contains(&bearing),
        "the sun moved on bearing {bearing}, which is the eastern half of the compass"
    );
}

/// Going to the moon and reloading used to drop you in the ocean: the code
/// named a column and every body shares the planet's sector grid, so the moon
/// read back as a place on the planet.
#[test]
fn a_link_from_the_moon_opens_on_the_moon() {
    let mut client = Client::new(Recipe::new(1)).expect("a world");
    let mut input = Input::default();
    client.visit_moon(150.0, -0.3, 8.0);
    let said = run(&mut client, &mut input, 6.0);
    assert!(
        said.place.starts_with('m'),
        "never reached the moon: {}",
        said.place
    );
    println!("a link from the moon: {}", said.pose);

    // A refresh is a fresh client handed whatever the address bar held.
    let mut fresh = Client::new(Recipe::new(1)).expect("a world");
    fresh.go_to(&said.pose).expect("the link opens");
    let there = run(&mut fresh, &mut input, 2.0);
    assert!(
        there.place.starts_with('m'),
        "the link landed on the planet: {} from {}",
        there.place,
        said.pose
    );
}

/// A link is for showing someone what you were looking at, so it has to carry
/// the aim and not only the spot.
#[test]
fn a_link_arrives_looking_the_way_it_was_sent() {
    let mut client = Client::new(Recipe::new(1)).expect("a world");
    let mut input = Input::default();
    // Turn somewhere that is not the spawn heading, and look down a little.
    run(&mut client, &mut input, 1.5);
    input.look = [900.0, 200.0];
    run(&mut client, &mut input, 0.2);
    input.look = [0.0, 0.0];
    let said = run(&mut client, &mut input, 1.5);
    let sent = said.bearing_deg.expect("a bearing");

    let mut fresh = Client::new(Recipe::new(1)).expect("a world");
    fresh.go_to(&said.pose).expect("the link opens");
    let there = run(&mut fresh, &mut input, 1.5);
    let arrived = there.bearing_deg.expect("a bearing");

    let off = (arrived - sent + 540.0) % 360.0 - 180.0;
    assert!(
        off.abs() < 2.0,
        "sent on {sent} degrees and arrived on {arrived}, {off} out, from {}",
        said.pose
    );
    assert_eq!(there.place, said.place, "the link moved the place too");
}
