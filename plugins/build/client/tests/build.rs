//! Building, as a front end asks for it and as a shell's keys do: the plugin
//! plugged into a client, in a world that says it is on, driven by hand made
//! server frames, the way a shell drives it with a socket.

use client::{Chord, Client, Event, Input, Key, Level, Recipe};
use protocol::server_message;
use scene::VolumeChange;
use serde_json::{Value, json};

fn server(message: server_message::Message) -> Vec<u8> {
    protocol::encode(&protocol::ServerMessage {
        message: Some(message),
    })
}

fn build_on() -> Vec<protocol::Plugin> {
    vec![protocol::Plugin {
        name: build_client::NAME.into(),
        version: build_client::VERSION,
    }]
}

/// A world that welcomes this client at a level, with building on.
fn welcome(client: &Client, level: protocol::Level) -> Vec<u8> {
    let recipe = client.recipe();
    server(server_message::Message::Welcome(protocol::Welcome {
        session: 1,
        recipe: Some(protocol::Recipe {
            seed: worldgen::format_seed(recipe.seed),
            generator_version: recipe.generator_version,
            params_json: serde_json::to_string(&recipe.params).unwrap(),
        }),
        peers: vec![],
        level: level.into(),
        plugins: build_on(),
    }))
}

/// A client standing on dry land, in the middle of a sector, with building
/// plugged in and no world yet.
fn alone() -> Client {
    let mut client = Client::new(Recipe::new(1)).expect("a world");
    client.plug(build_client::plugin());
    client.go_to("4-77AEYRG").expect("a place to stand");
    client.update(1.0 / 60.0, &mut Input::default());
    client.drain_events();
    client.drain_volume_changes();
    client
}

/// The same, welcomed by a world as a builder. The world of these tests
/// says hello and no more: what it answers a change with is tried in
/// `kept.rs`, over the server's own module.
fn builder() -> Client {
    let mut client = alone();
    client.link_opened();
    client.receive(&welcome(&client, protocol::Level::Builder));
    client.drain_events();
    client
}

/// The same with no world, standing in for one: the cells are held by the
/// client alone, as in a headless picture.
fn rehearsed() -> Client {
    let mut client = alone();
    client.rehearse(Level::Builder);
    client.drain_events();
    client
}

/// Lives until the platform asked for is laid.
fn laid(client: &mut Client) {
    for _ in 0..10 {
        client.update(1.0 / 60.0, &mut Input::default());
    }
    let feet = client.host("build").feet();
    assert!(client.cells().covers(feet.point), "a platform is laid");
}

/// What building said since the last call, each event under its name
/// without the plugin's.
fn said(client: &mut Client) -> Vec<(String, Value)> {
    client
        .drain_events()
        .into_iter()
        .filter_map(|event| match event {
            Event::Plugin(event) => {
                let kind = event["type"].as_str()?.strip_prefix("build.")?.to_owned();
                Some((kind, event))
            }
            _ => None,
        })
        .collect()
}

fn refusals(client: &mut Client) -> Vec<String> {
    said(client)
        .into_iter()
        .filter(|(kind, _)| kind == "refused")
        .map(|(_, event)| event["reason"].as_str().unwrap_or_default().to_owned())
        .collect()
}

fn rejected(client: &mut Client) -> bool {
    let rejected = |event: &Event| matches!(event, Event::Rejected { .. });
    client.drain_events().iter().any(rejected)
}

fn press(client: &mut Client, code: &str, chord: Chord) {
    let mut input = Input::default();
    input.code(code, chord, true);
    client.update(1.0 / 60.0, &mut input);
}

const TAKE_CREATE: &str = r#"{"type":"build.take","tool":"create"}"#;
const TAKE_PLATFORM: &str = r#"{"type":"build.take","tool":"platform"}"#;
const LAY: &str = r#"{"type":"build.lay"}"#;

#[test]
fn a_tool_in_hand_builds_nothing() {
    let mut client = builder();
    client.command_json(TAKE_CREATE);
    assert_eq!(
        said(&mut client),
        vec![(
            "hand".to_owned(),
            json!({
                "type": "build.hand", "tool": "create", "paint": 0,
                "finish": "matte", "edge": "none", "platform": 16
            })
        )]
    );
    client.update(1.0 / 60.0, &mut Input::default());
    assert!(client.pointing());
    assert!(refusals(&mut client).is_empty());
    let built = |change: &VolumeChange| matches!(change, VolumeChange::Add(..));
    assert!(!client.drain_volume_changes().iter().any(built));
}

#[test]
fn a_platform_is_laid_where_it_is_asked_for() {
    let mut client = builder();
    client.command_json(r#"{"type":"build.platform","side":16}"#);
    client.command_json(r#"{"type":"build.lay","base":"deck"}"#);
    assert!(refusals(&mut client).is_empty());
    // It is laid and drawn over the next few frames, none of them at once.
    let built = |change: &VolumeChange| matches!(change, VolumeChange::Add(..));
    assert!(!client.drain_volume_changes().iter().any(built));
    let mut drawn = false;
    for _ in 0..10 {
        client.update(1.0 / 60.0, &mut Input::default());
        drawn |= client.drain_volume_changes().iter().any(built);
    }
    assert!(drawn);
    // Asking lays it with no tool in hand, and takes none.
    assert!(!client.pointing());
    // What was asked of the world went up in the envelope of the cells: the
    // volume opened, and the platform as one change.
    let asked: Vec<String> = client
        .drain_outbound()
        .into_iter()
        .filter_map(|out| match out {
            client::Outbound::Frame(frame) => {
                match protocol::decode::<protocol::ClientMessage>(&frame)
                    .ok()?
                    .message?
                {
                    protocol::client_message::Message::Envelope(envelope) => {
                        (envelope.plugin == "cells" && envelope.kind != "look")
                            .then_some(envelope.kind)
                    }
                    _ => None,
                }
            }
            client::Outbound::Close => None,
        })
        .collect();
    assert_eq!(asked, ["open", "change"]);

    // With no world, the client holds the cells alone and what was laid is
    // its own change to take back, which building says.
    let mut client = rehearsed();
    client.command_json(LAY);
    laid(&mut client);
    let history = json!({"type": "build.history", "undo": true, "redo": false});
    assert!(said(&mut client).contains(&("history".to_owned(), history)));
}

#[test]
fn the_world_says_who_builds() {
    // With no world there is no building to ask (DECISIONS 104).
    let mut client = alone();
    client.command_json(TAKE_CREATE);
    assert!(!client.pointing());
    assert!(rejected(&mut client));

    // A world that welcomes a visitor refuses the tool, and the platform.
    client.link_opened();
    client.receive(&welcome(&client, protocol::Level::Anonymous));
    client.drain_events();
    client.command_json(TAKE_CREATE);
    assert!(!client.pointing());
    assert_eq!(refusals(&mut client), ["level"]);
    client.command_json(LAY);
    assert_eq!(refusals(&mut client), ["level"]);

    // A dropped link takes building with it: offline is no way to a tool.
    client.link_closed();
    client.drain_events();
    client.command_json(TAKE_CREATE);
    assert!(!client.pointing());
    assert!(rejected(&mut client));

    // A builder's welcome hands the tools over.
    client.link_opened();
    client.receive(&welcome(&client, protocol::Level::Builder));
    client.command_json(TAKE_CREATE);
    assert!(client.pointing());
}

#[test]
fn building_hears_the_keys_it_asked_for_by_name() {
    let mut client = rehearsed();
    let plain = Chord::default();
    assert!(client.asks("KeyB", plain) && client.asks("Escape", plain));
    assert!(
        !client.asks("KeyW", plain),
        "a key of the core is the core's"
    );

    // B starts building with the tool taken last, and stops it.
    press(&mut client, "KeyB", plain);
    assert!(client.pointing());
    press(&mut client, "Digit2", plain);
    let hands = said(&mut client);
    assert_eq!(hands.last().unwrap().1["tool"], "delete");
    press(&mut client, "KeyB", plain);
    assert!(!client.pointing());
    press(&mut client, "KeyB", plain);
    assert_eq!(said(&mut client).last().unwrap().1["tool"], "delete");
    // Escape puts the tool down.
    press(&mut client, "Escape", plain);
    assert!(!client.pointing());

    // The chord takes back a platform, and puts it back.
    client.command_json(LAY);
    laid(&mut client);
    let command = |shift| Chord {
        command: true,
        shift,
    };
    assert!(client.asks("KeyZ", command(false)) && !client.asks("KeyZ", plain));
    press(&mut client, "KeyZ", command(false));
    assert_eq!(client.cells().history(), (false, true));
    press(&mut client, "KeyZ", command(true));
    assert_eq!(client.cells().history(), (true, false));
    press(&mut client, "KeyZ", command(false));
    press(&mut client, "KeyY", command(false));
    assert_eq!(client.cells().history(), (true, false));
}

#[test]
fn a_drag_of_the_pointer_lays_cells_with_a_tool_in_hand() {
    let mut client = rehearsed();
    client.command_json(LAY);
    laid(&mut client);
    // Looking down at the slab under the feet, from a little over it.
    client.pose(6.0, -1.2, 0.0);
    client.command_json(TAKE_CREATE);
    let mut input = Input::default();
    input.pointer = Some([0.5, 0.5]);
    client.update(1.0 / 60.0, &mut input);
    assert!(
        client.cells().ghost().is_some(),
        "the slab is under the pointer"
    );
    input.key(Key::Use, true);
    client.update(1.0 / 60.0, &mut input);
    input.pointer = Some([0.6, 0.5]);
    client.update(1.0 / 60.0, &mut input);
    input.key(Key::Use, false);
    client.update(1.0 / 60.0, &mut input);
    // The stroke landed: a second change to take back, over the platform.
    client.command_json(r#"{"type":"build.undo"}"#);
    assert_eq!(client.cells().history(), (true, true));
}

#[test]
fn a_world_that_switches_building_off_takes_the_tool_and_leaves_what_stands() {
    let mut client = builder();
    client.command_json(LAY);
    laid(&mut client);
    client.command_json(TAKE_CREATE);
    assert!(client.pointing());
    let off = protocol::Plugins { plugins: vec![] };
    client.receive(&server(server_message::Message::Plugins(off)));
    assert!(!client.pointing());
    assert!(!client.asks("KeyB", Chord::default()));
    // What was built is the core's, and stands with building off.
    let frame = client.settled_frame();
    assert!(!frame.volumes.is_empty());
    assert_eq!(frame.ghost, None);
    // And on again, the hand is empty and says so when asked.
    let on = protocol::Plugins {
        plugins: build_on(),
    };
    client.receive(&server(server_message::Message::Plugins(on)));
    client.drain_events();
    client.command_json(r#"{"type":"build.state"}"#);
    let kinds: Vec<String> = said(&mut client)
        .into_iter()
        .map(|(kind, _)| kind)
        .collect();
    assert_eq!(kinds, ["hand", "over", "history"]);
}

/// Lives a few frames with nothing held down.
fn live(client: &mut Client) {
    for _ in 0..10 {
        client.update(1.0 / 60.0, &mut Input::default());
    }
}

#[test]
fn a_tool_in_hand_shows_where_cells_are() {
    let mut client = rehearsed();
    let feet = client.host("build").feet();
    assert!(client.cells().guides().is_empty());

    // A tool that makes strokes shows no platform: with nothing built
    // there is nothing to show.
    client.command_json(r#"{"type":"build.paint","paint":4}"#);
    client.command_json(TAKE_CREATE);
    live(&mut client);
    assert!(client.cells().guides().is_empty());

    // The platform tool shows the slab one would lay, in the colour in
    // hand, as wide as the side picked. Its pointer makes no stroke.
    client.command_json(TAKE_PLATFORM);
    live(&mut client);
    assert!(client.pointing());
    assert_eq!(client.cells().ghost(), None);
    let [slab] = client.cells().guides() else {
        panic!("one guide: {:?}", client.cells().guides());
    };
    assert_eq!(slab.paint, Some(4));
    assert_eq!(slab.span.size(), [16, 16, 1]);
    assert!(
        slab.span
            .contains([feet.point.u as i32, feet.point.v as i32, slab.span.min[2]])
    );
    assert_eq!(client.settled_frame().guides.len(), 1);
    client.command_json(r#"{"type":"build.platform","side":64}"#);
    live(&mut client);
    assert_eq!(client.cells().guides()[0].span.size(), [64, 64, 1]);

    // The platform lands where its slab was shown. Then the body is in a
    // volume, shown as room to build in, and there is no slab left to lay.
    let shown = client.cells().guides()[0].span;
    client.command_json(LAY);
    laid(&mut client);
    live(&mut client);
    assert!(!client.cells().cell(feet.point.sector, shown.min).is_air());
    let [room] = client.cells().guides() else {
        panic!("one guide: {:?}", client.cells().guides());
    };
    assert_eq!(room.paint, None);
    assert_eq!(Some(room.span), client.cells().bounds_over(feet.point));
    let over = json!({"type": "build.over", "volume": true});
    assert!(said(&mut client).contains(&("over".to_owned(), over)));

    // A hole in the slab is a cell a platform would make: the slab shows,
    // with the platform tool and with no other.
    let hole = voxel::Gesture::Delete {
        span: voxel::Span::cell(shown.min),
    };
    assert_eq!(
        client.host("build").apply(feet.point.sector, &[hole]),
        Ok(true)
    );
    live(&mut client);
    assert_eq!(client.cells().guides().len(), 2);
    client.command_json(TAKE_CREATE);
    live(&mut client);
    assert_eq!(client.cells().guides().len(), 1);

    // With the tool put down nothing is shown.
    client.command_json(r#"{"type":"build.take","tool":null}"#);
    assert!(client.cells().guides().is_empty());
    assert!(client.settled_frame().guides.is_empty());
}

#[test]
fn a_volume_is_closed_with_all_built_in_it_and_stands_again() {
    let mut client = rehearsed();
    let feet = client.host("build").feet();
    // With nothing built there is none to close.
    client.command_json(r#"{"type":"build.close"}"#);
    assert_eq!(refusals(&mut client), ["empty"]);

    client.command_json(LAY);
    laid(&mut client);
    client.command_json(TAKE_CREATE);
    live(&mut client);
    client.drain_events();
    let held = client.cells().bounds_over(feet.point).expect("a volume");
    let slab = client.host("build").feet();
    let under = [
        slab.point.u as i32,
        slab.point.v as i32,
        (slab.height_m / topology::BLOCK_M).round() as i32 - 1,
    ];
    assert!(!client.cells().cell(feet.point.sector, under).is_air());

    client.command_json(r#"{"type":"build.close"}"#);
    assert!(refusals(&mut client).is_empty());
    assert!(!client.cells().covers(feet.point));
    assert!(client.cells().cell(feet.point.sector, under).is_air());
    live(&mut client);
    let over = json!({"type": "build.over", "volume": false});
    assert!(said(&mut client).contains(&("over".to_owned(), over)));
    assert!(client.settled_frame().volumes.is_empty());

    // It is a change like any other: taken back, the volume stands as it
    // was, and put back, it is gone again.
    client.command_json(r#"{"type":"build.undo"}"#);
    assert_eq!(client.cells().bounds_over(feet.point), Some(held));
    assert!(!client.cells().cell(feet.point.sector, under).is_air());
    assert!(!client.settled_frame().volumes.is_empty());
    client.command_json(r#"{"type":"build.redo"}"#);
    assert!(!client.cells().covers(feet.point));
    assert_eq!(client.cells().history(), (true, false));

    // A visitor closes nothing.
    let mut client = alone();
    client.link_opened();
    client.receive(&welcome(&client, protocol::Level::Anonymous));
    client.drain_events();
    client.command_json(r#"{"type":"build.close"}"#);
    assert_eq!(refusals(&mut client), ["level"]);
}

#[test]
fn what_is_laid_is_a_colour_in_a_finish_with_an_edge() {
    let mut client = rehearsed();
    let feet = client.host("build").feet();
    // Building starts with the platform tool: a stroke starts on what is
    // built. The fourth key takes it too.
    press(&mut client, "KeyB", Chord::default());
    assert_eq!(said(&mut client).last().unwrap().1["tool"], "platform");
    press(&mut client, "Digit1", Chord::default());
    press(&mut client, "Digit4", Chord::default());
    assert_eq!(said(&mut client).last().unwrap().1["tool"], "platform");

    // A platform of glass with a white edge, in the colour in hand.
    client.command_json(r#"{"type":"build.paint","paint":11}"#);
    client.command_json(r#"{"type":"build.finish","finish":"glass"}"#);
    client.command_json(r#"{"type":"build.edge","edge":"white"}"#);
    let hand = said(&mut client).last().unwrap().1.clone();
    assert_eq!(
        (&hand["finish"], &hand["edge"]),
        (&json!("glass"), &json!("white"))
    );
    // A slab alone, with nothing under it.
    client.command_json(r#"{"type":"build.lay","base":"floating"}"#);
    laid(&mut client);
    let slab = client.host("build").feet();
    let under = [
        slab.point.u as i32,
        slab.point.v as i32,
        (slab.height_m / topology::BLOCK_M).round() as i32 - 1,
    ];
    let paint = client.cells().cell(feet.point.sector, under).paint();
    let paint = voxel::Paint::of(paint.expect("the slab"));
    assert_eq!(
        (paint.color, paint.finish, paint.edge),
        (11, voxel::Finish::Glass, voxel::Edge::White)
    );
    // Glass is drawn apart from the cubes, over them, and holds the body.
    let frame = client.settled_frame();
    assert!(frame.volumes.is_empty() && !frame.glass.is_empty());
    assert!(frame.lamps.is_empty());

    // Repainted as a light, the same cells shine: a lamp where they are,
    // in their colour, for whatever is near.
    let held = client.cells().bounds_over(feet.point).unwrap();
    let repaint = voxel::Gesture::Paint {
        span: voxel::Span::between(
            [held.min[0], held.min[1], under[2]],
            [held.max[0], held.max[1], under[2]],
        ),
        paint: voxel::Paint {
            color: 4,
            finish: voxel::Finish::Light,
            edge: voxel::Edge::None,
        }
        .byte(),
    };
    assert_eq!(
        client.host("build").apply(feet.point.sector, &[repaint]),
        Ok(true)
    );
    let frame = client.settled_frame();
    assert!(frame.glass.is_empty() && !frame.volumes.is_empty());
    assert!(!frame.lamps.is_empty());
    let lamp = frame.lamps[0];
    assert!(lamp.color.x > lamp.color.z, "a red light: {lamp:?}");
    assert!(lamp.reach_m >= 8.0);
}
