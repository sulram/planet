//! First person: the boom pulled all the way in, by the wheel or the key,
//! the own body out of the view, and a link that keeps it.

use client::{Client, Event, Frame, Input, Key, Recipe};

/// Lives `seconds` and says the last pose the client said, and the last
/// frame.
fn live(client: &mut Client, input: &mut Input, seconds: f64) -> (String, Frame) {
    let mut pose = None;
    let mut frame = None;
    for _ in 0..(seconds * 60.0) as u32 {
        frame = Some(client.update(1.0 / 60.0, input));
        for event in client.drain_events() {
            if let Event::Stats { pose: said, .. } = event {
                pose = Some(said);
            }
        }
    }
    let pose = pose.expect("the client says where it is");
    (pose, frame.expect("a frame"))
}

/// Whether the own body is seen: the box figure, as no avatar has come.
fn seen(frame: &Frame) -> bool {
    assert!(!frame.boxes.is_empty(), "a body is drawn");
    let unseen = frame.boxes.iter().filter(|part| part.shadow_only).count();
    assert!(
        unseen == 0 || unseen == frame.boxes.len(),
        "all of it or none"
    );
    unseen == 0
}

fn first_person(pose: &str) -> bool {
    pose.ends_with(",0")
}

#[test]
fn a_line_past_the_nearest_boom_lands_at_the_eye_and_a_line_out_comes_back() {
    let mut client = Client::new(Recipe::new(1)).expect("a world");
    let mut input = Input::default();
    let (pose, frame) = live(&mut client, &mut input, 1.0);
    assert!(seen(&frame) && !first_person(&pose), "{pose}");

    // All the way in to the nearest third person boom: still out.
    input.zoom = 30.0;
    let (pose, frame) = live(&mut client, &mut input, 0.5);
    assert!(seen(&frame) && !first_person(&pose), "{pose}");

    // A line more, and well under a second later the camera is in the head.
    input.zoom = 1.0;
    let (pose, frame) = live(&mut client, &mut input, 0.5);
    assert!(!seen(&frame) && first_person(&pose), "{pose}");

    input.zoom = -1.0;
    let (pose, frame) = live(&mut client, &mut input, 0.5);
    assert!(seen(&frame) && !first_person(&pose), "{pose}");
}

#[test]
fn the_key_swings_to_the_eye_and_back() {
    let mut client = Client::new(Recipe::new(1)).expect("a world");
    let mut input = Input::default();
    live(&mut client, &mut input, 1.0);
    for eye in [true, false, true] {
        input.key(Key::FirstPerson, true);
        let (pose, frame) = live(&mut client, &mut input, 0.5);
        assert_eq!(seen(&frame), !eye);
        assert_eq!(first_person(&pose), eye, "{pose}");
    }
}

#[test]
fn a_link_copied_in_first_person_opens_in_first_person() {
    let mut client = Client::new(Recipe::new(1)).expect("a world");
    let mut input = Input::default();
    input.key(Key::FirstPerson, true);
    let (sent, _) = live(&mut client, &mut input, 1.0);
    assert!(first_person(&sent), "{sent}");

    let mut fresh = Client::new(Recipe::new(1)).expect("a world");
    fresh.go_to(&sent).expect("the link opens");
    let (there, frame) = live(&mut fresh, &mut input, 1.0);
    assert!(!seen(&frame) && first_person(&there), "{there}");

    // A link that says nothing of the boom, as a portal's place, leaves it.
    let place = sent.trim_end_matches(",0");
    fresh.go_to(place).expect("the place opens");
    let (there, frame) = live(&mut fresh, &mut input, 1.0);
    assert!(!seen(&frame) && first_person(&there), "{there}");
}
