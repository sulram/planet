//! The client says when the view is drawn whole, and again after a leap.

use client::{Client, Command, Event, Input, Recipe};

/// The frame in which the client said `Settled`, within `seconds`, or none.
fn settled_within(client: &mut Client, input: &mut Input, seconds: f64) -> Option<u32> {
    for frame in 0..(seconds * 60.0) as u32 {
        client.update(1.0 / 60.0, input);
        for event in client.drain_events() {
            assert!(!matches!(event, Event::Rejected { .. }), "{event:?}");
            if event == Event::Settled {
                return Some(frame);
            }
        }
    }
    None
}

#[test]
fn a_view_settles_once_and_a_leap_unsettles_it() {
    let mut client = Client::new(Recipe::new(1)).expect("a world");
    let mut input = Input::default();
    let first = settled_within(&mut client, &mut input, 30.0).expect("the first view settles");
    assert!(first > 0, "a view is not drawn whole in its first frame");
    // Standing still, it is not said again.
    assert_eq!(settled_within(&mut client, &mut input, 2.0), None);
    // A leap is a new view to build: unsettled, then settled once more.
    client.command(Command::GoTo {
        place: "3-K7M42Q".into(),
    });
    assert!(
        settled_within(&mut client, &mut input, 30.0).is_some(),
        "the view settles again after a leap"
    );
}
