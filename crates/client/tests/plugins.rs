//! The host of plugins: what a shell plugged in is off until a world says it
//! is on, and then a command and a frame reach it by its name.

use client::{Client, Command, Event, Host, Mode, Outbound, Plugin, PluginOn, Recipe};
use protocol::{Message, client_message, server_message};
use serde_json::{Value, json};

/// A plugin that sends up what it is told to shout and says over the seam
/// what it hears.
struct Echo;

impl Plugin for Echo {
    fn name(&self) -> &'static str {
        "echo"
    }

    fn version(&self) -> u32 {
        2
    }

    fn command(&mut self, kind: &str, body: Value, host: &mut Host<'_>) {
        match (kind, body.get("text").and_then(Value::as_str)) {
            ("shout", Some(text)) => host.send("shout", text.as_bytes().to_vec()),
            _ => host.reject(format!("`{kind}` is no command of echo")),
        }
    }

    fn receive(&mut self, kind: &str, payload: &[u8], host: &mut Host<'_>) {
        host.emit(kind, &json!({ "text": String::from_utf8_lossy(payload) }));
    }
}

fn server(message: server_message::Message) -> Vec<u8> {
    protocol::encode(&protocol::ServerMessage {
        message: Some(message),
    })
}

fn welcome(client: &Client, plugins: Vec<protocol::Plugin>) -> Vec<u8> {
    let recipe = client.recipe();
    server(server_message::Message::Welcome(protocol::Welcome {
        session: 1,
        recipe: Some(protocol::Recipe {
            seed: worldgen::format_seed(recipe.seed),
            generator_version: recipe.generator_version,
            params_json: serde_json::to_string(&recipe.params).unwrap(),
        }),
        peers: vec![],
        level: protocol::Level::Anonymous.into(),
        plugins,
    }))
}

fn echo(version: u32) -> protocol::Plugin {
    protocol::Plugin {
        name: "echo".into(),
        version,
    }
}

fn heard(plugin: &str, kind: &str, text: &str) -> Vec<u8> {
    server(server_message::Message::Envelope(protocol::Envelope {
        plugin: plugin.into(),
        kind: kind.into(),
        payload: text.as_bytes().to_vec(),
    }))
}

fn envelopes(client: &mut Client) -> Vec<protocol::Envelope> {
    client
        .drain_outbound()
        .into_iter()
        .filter_map(|out| match out {
            Outbound::Frame(frame) => {
                match protocol::ClientMessage::decode(&frame[..]).unwrap().message {
                    Some(client_message::Message::Envelope(envelope)) => Some(envelope),
                    _ => None,
                }
            }
            Outbound::Close => None,
        })
        .collect()
}

fn rejected(events: &[Event]) -> usize {
    events
        .iter()
        .filter(|event| matches!(event, Event::Rejected { .. }))
        .count()
}

fn plugged() -> Client {
    let mut client = Client::new(Recipe::new(1)).unwrap();
    client.plug(Box::new(Echo));
    client.drain_events();
    client
}

const SHOUT: &str = r#"{"type":"echo.shout","text":"hi"}"#;

#[test]
fn a_plugin_is_off_until_a_world_says_it_is_on() {
    let mut client = plugged();
    client.command_json(SHOUT);
    assert_eq!(
        rejected(&client.drain_events()),
        1,
        "offline, nothing is on"
    );
    assert!(envelopes(&mut client).is_empty());

    client.link_opened();
    client.receive(&welcome(&client, vec![echo(2)]));
    assert!(
        client.drain_events().contains(&Event::Statement {
            plugins: vec![PluginOn {
                name: "echo".into(),
                version: 2
            }]
        }),
        "the world's statement goes over the seam"
    );

    client.command_json(SHOUT);
    assert_eq!(
        envelopes(&mut client),
        vec![protocol::Envelope {
            plugin: "echo".into(),
            kind: "shout".into(),
            payload: b"hi".to_vec()
        }],
        "a command with the plugin's name goes up in its envelope"
    );
    client.command_json(r#"{"type":"echo.whisper","text":"hi"}"#);
    client.command_json(r#"{"type":"nobody.shout","text":"hi"}"#);
    assert_eq!(
        rejected(&client.drain_events()),
        2,
        "the plugin refuses its own, the host what names nobody"
    );

    client.receive(&heard("echo", "shouted", "yo"));
    client.receive(&heard("nobody", "shouted", "lost"));
    assert_eq!(
        client.drain_events(),
        vec![Event::Plugin(
            json!({ "type": "echo.shouted", "text": "yo" })
        )],
        "what the server says in an envelope reaches the plugin it names"
    );
}

#[test]
fn the_world_switches_a_plugin_and_a_dropped_link_hushes_it() {
    let mut client = plugged();
    client.link_opened();
    client.receive(&welcome(&client, vec![echo(2)]));
    client.drain_events();

    client.receive(&server(server_message::Message::Plugins(
        protocol::Plugins { plugins: vec![] },
    )));
    assert_eq!(
        client.drain_events(),
        vec![Event::Statement { plugins: vec![] }]
    );
    client.command_json(SHOUT);
    client.receive(&heard("echo", "shouted", "yo"));
    assert_eq!(
        rejected(&client.drain_events()),
        1,
        "off, a command is refused and a frame let pass"
    );
    assert!(envelopes(&mut client).is_empty());

    client.receive(&server(server_message::Message::Plugins(
        protocol::Plugins {
            plugins: vec![echo(2)],
        },
    )));
    client.drain_events();
    client.link_closed();
    assert!(
        client
            .drain_events()
            .contains(&Event::Statement { plugins: vec![] }),
        "no world, no plugin on"
    );
}

#[test]
fn a_world_that_speaks_another_version_leaves_the_plugin_off() {
    let mut client = plugged();
    client.link_opened();
    client.receive(&welcome(&client, vec![echo(3)]));
    let events = client.drain_events();
    assert!(events.contains(&Event::Statement { plugins: vec![] }));
    assert_eq!(rejected(&events), 1, "and the log says which versions");
}

#[test]
fn the_cores_own_commands_take_the_same_door() {
    let mut client = plugged();
    client.command_json(r#"{"type":"set_mode","mode":"fly"}"#);
    assert_eq!(
        client.drain_events(),
        vec![Event::ModeChanged { mode: Mode::Fly }]
    );
    client.command(Command::SetMode { mode: Mode::Walk });
    client.drain_events();
    client.command_json(r#"{"type":"from_the_future"}"#);
    client.command_json("not json");
    assert_eq!(rejected(&client.drain_events()), 2);
}
