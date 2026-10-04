//! Building, as a front end asks for it and as a shell's keys do: the plugin
//! plugged into a client, in a world that says it is on, driven by hand made
//! server frames, the way a shell drives it with a socket.

use client::{Chord, Client, Event, Input, Key, Recipe};
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

/// The same, welcomed by a world as a builder.
fn builder() -> Client {
    let mut client = alone();
    client.link_opened();
    client.receive(&welcome(&client, protocol::Level::Builder));
    client.drain_events();
    client
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
const LAY: &str = r#"{"type":"build.lay"}"#;

#[test]
fn a_tool_in_hand_builds_nothing() {
    let mut client = builder();
    client.command_json(TAKE_CREATE);
    assert_eq!(
        said(&mut client),
        vec![(
            "hand".to_owned(),
            json!({"type": "build.hand", "tool": "create", "paint": 0, "platform": 16})
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
    // It is a change to take back, and building says so.
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
    let mut client = builder();
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
    while client.cells().history() != (true, false) {
        client.update(1.0 / 60.0, &mut Input::default());
    }
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
    let mut client = builder();
    client.command_json(LAY);
    while client.cells().history() != (true, false) {
        client.update(1.0 / 60.0, &mut Input::default());
    }
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
    while client.cells().history() != (true, false) {
        client.update(1.0 / 60.0, &mut Input::default());
    }
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
    assert_eq!(kinds, ["hand", "history"]);
}
