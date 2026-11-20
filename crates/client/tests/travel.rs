//! Travelling to a place chosen inside the world goes through the passage a
//! portal takes a body by, with no portal: the picture drawn in, held while
//! the ground at the place comes, and burst out there.

use client::{Client, Command, Event, Input, Recipe};

/// A client standing in its first view, drawn whole.
fn standing() -> (Client, Input) {
    let mut client = Client::new(Recipe::new(1)).expect("a world");
    client.set_view([1600, 900]);
    let mut input = Input::default();
    for _ in 0..60 {
        client.update(1.0 / 60.0, &mut input);
    }
    client.drain_events();
    (client, input)
}

fn here(client: &Client) -> String {
    client.here().text(client.generator().sphere().blocks())
}

#[test]
fn a_travel_draws_the_picture_in_holds_it_and_bursts_out_at_the_place() {
    let (mut client, mut input) = standing();
    let before = here(&client);
    client.command(Command::Travel {
        place: "3-K7M42Q".into(),
    });
    let warp = client
        .update(1.0 / 60.0, &mut input)
        .warp
        .expect("the picture is drawn into the body");
    assert!(warp.phase < 0.0, "falling in: {warp:?}");
    assert_eq!(here(&client), before, "it falls in where it stood");
    // In, held, out: the picture is drawn in and let go, never back.
    let (mut last, mut held, mut frames) = (-1.0, false, 0);
    let mut settled = false;
    while let Some(warp) = client.update(1.0 / 60.0, &mut input).warp {
        assert!(warp.phase >= last, "{last} then {}", warp.phase);
        held |= warp.phase == 0.0;
        last = warp.phase;
        frames += 1;
        assert!(frames < 60 * 10, "the passage ends");
        for event in client.drain_events() {
            assert!(!matches!(event, Event::Rejected { .. }), "{event:?}");
            settled |= event == Event::Settled;
        }
    }
    assert!(held && last > 0.0, "held, then burst out");
    assert!(settled, "held until the ground at the place came");
    let landed = here(&client);
    assert!(landed.starts_with("3-K7M42Q"), "landed at {landed}");
}

#[test]
fn a_travel_taken_over_goes_on_from_as_deep_as_the_picture_is() {
    let (mut client, mut input) = standing();
    client.command(Command::Travel {
        place: "3-K7M42Q".into(),
    });
    // Run until the picture is bursting out, then travel again: it falls
    // in from as deep as it was, and lands at the second place.
    let mut last = -1.0;
    while last <= 0.3 {
        last = client
            .update(1.0 / 60.0, &mut input)
            .warp
            .expect("still passing")
            .phase;
    }
    client.command(Command::Travel {
        place: "5-77AEYRG".into(),
    });
    let next = client
        .update(1.0 / 60.0, &mut input)
        .warp
        .expect("passing again")
        .phase;
    assert!(next < 0.0, "falling in again: {next}");
    assert!(
        (next.abs() - last.abs()).abs() < 0.1,
        "as deep as it was: {last} then {next}"
    );
    let mut frames = 0;
    while client.update(1.0 / 60.0, &mut input).warp.is_some() {
        frames += 1;
        assert!(frames < 60 * 10, "the passage ends");
    }
    let landed = here(&client);
    assert!(landed.starts_with("5-77AEYRG"), "landed at {landed}");
}

#[test]
fn a_place_that_cannot_be_read_is_refused_and_the_body_stays() {
    let (mut client, mut input) = standing();
    let before = here(&client);
    client.command(Command::Travel {
        place: "nowhere".into(),
    });
    let frame = client.update(1.0 / 60.0, &mut input);
    assert!(frame.warp.is_none(), "no passage to nowhere");
    assert!(
        client
            .drain_events()
            .iter()
            .any(|event| matches!(event, Event::Rejected { .. })),
        "refused"
    );
    assert_eq!(here(&client), before);
}
