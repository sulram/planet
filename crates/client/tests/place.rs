//! Saying where you are: the code a person reads out, and the way they face.

use client::{Client, Event, Input, Key, Recipe};

/// Everything the HUD shows about a place, as the client last said it.
struct Said {
    place: String,
    bearing_deg: Option<f64>,
}

fn run(client: &mut Client, input: &mut Input, seconds: f64) -> Said {
    let mut said = None;
    for _ in 0..(seconds * 60.0) as u32 {
        client.update(1.0 / 60.0, input);
        for event in client.drain_events() {
            if let Event::Stats {
                place, bearing_deg, ..
            } = event
            {
                said = Some(Said { place, bearing_deg });
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
        code.chars().all(|c| "0123456789ABCDEFGHJKMNPQRSTVWXYZ".contains(c)),
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

    assert_ne!(before.place, after.place, "six seconds of walking moved nobody");
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
    let before = run(&mut client, &mut input, 3.0).bearing_deg.expect("a bearing");

    // Half a turn of the view, then long enough for the body to follow it.
    input.look = [1800.0, 0.0];
    run(&mut client, &mut input, 1.0);
    input.look = [0.0, 0.0];
    let after = run(&mut client, &mut input, 3.0).bearing_deg.expect("a bearing");

    let turned = (after - before + 360.0) % 360.0;
    assert!(
        (90.0..270.0).contains(&turned),
        "half a turn read as {turned} degrees, from {before} to {after}"
    );
}
