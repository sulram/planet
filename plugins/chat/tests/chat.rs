//! A line goes out and comes back: the client's side of chat, driven by hand
//! made server frames, the way a shell drives it with a socket.

use chat::wire;
use client::{Client, Command, Event, Outbound, Recipe};
use protocol::{Message, client_message, server_message};
use serde_json::json;

fn server(message: server_message::Message) -> Vec<u8> {
    protocol::encode(&protocol::ServerMessage {
        message: Some(message),
    })
}

/// A world that welcomes this client with chat on.
fn welcome(client: &Client) -> Vec<u8> {
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
        plugins: vec![protocol::Plugin {
            name: chat::NAME.into(),
            version: chat::VERSION,
        }],
    }))
}

fn stance() -> protocol::Stance {
    protocol::Stance {
        sector: 0,
        u: 30_000.0,
        v: 30_000.0,
        height_m: 5.0,
        facing_z: -1.0,
        gait: protocol::Gait::Walk.into(),
        speed_mps: 1.5,
        ..Default::default()
    }
}

fn said(stance: Option<protocol::Stance>) -> Vec<u8> {
    server(server_message::Message::Envelope(protocol::Envelope {
        plugin: chat::NAME.into(),
        kind: "said".into(),
        payload: wire::Said {
            session: 2,
            scope: wire::Scope::Near.into(),
            text: "here".into(),
            stance,
        }
        .encode_to_vec(),
    }))
}

/// What chat sent up since the last call.
fn sent(client: &mut Client) -> Vec<wire::Say> {
    client
        .drain_outbound()
        .into_iter()
        .filter_map(|out| match out {
            Outbound::Frame(frame) => {
                match protocol::ClientMessage::decode(&frame[..]).unwrap().message {
                    Some(client_message::Message::Envelope(envelope)) => {
                        assert_eq!(
                            (envelope.plugin.as_str(), envelope.kind.as_str()),
                            ("chat", "say")
                        );
                        Some(wire::Say::decode(&envelope.payload[..]).unwrap())
                    }
                    _ => None,
                }
            }
            Outbound::Close => None,
        })
        .collect()
}

/// The place of the last line heard.
fn place(client: &mut Client) -> String {
    match client.drain_events().pop() {
        Some(Event::Plugin(event)) => event["place"].as_str().expect("a place").to_owned(),
        other => panic!("a line heard is an event of chat: {other:?}"),
    }
}

#[test]
fn a_line_goes_out_online_and_comes_back_with_a_place() {
    let mut client = Client::new(Recipe::new(1)).unwrap();
    client.plug(chat::plugin());
    client.command_json(r#"{"type":"chat.say","scope":"world","text":"anyone?"}"#);
    assert!(
        sent(&mut client).is_empty(),
        "a line said offline goes nowhere"
    );

    client.link_opened();
    client.receive(&welcome(&client));
    client.command_json(r#"{"type":"chat.say","scope":"near","text":"hi","here":true}"#);
    client.command_json(r#"{"type":"chat.say","scope":"world","text":"all"}"#);
    let out = sent(&mut client);
    assert!(
        matches!(&out[..], [near, world]
            if near.scope() == wire::Scope::Near && near.text == "hi" && near.here
            && world.scope() == wire::Scope::World && world.text == "all" && !world.here),
        "{out:?}"
    );

    client.drain_events();
    client.receive(&said(None));
    assert_eq!(
        client.drain_events(),
        vec![Event::Plugin(json!({
            "type": "chat.said",
            "session": 2,
            "scope": "near",
            "text": "here",
            "place": null
        }))]
    );

    // A place shared on the ground has no height; one shared in the air keeps
    // it. Either is a place `GoTo` accepts.
    client.receive(&said(Some(stance())));
    let ground = place(&mut client);
    assert!(!ground.contains('@'), "{ground}");
    client.receive(&said(Some(protocol::Stance {
        gait: protocol::Gait::Fly.into(),
        height_m: 40.0,
        ..stance()
    })));
    let aloft = place(&mut client);
    assert!(
        aloft.starts_with(&ground) && aloft.ends_with("@80"),
        "{aloft}"
    );
    client.command(Command::GoTo { place: aloft });
    assert!(
        !client
            .drain_events()
            .iter()
            .any(|e| matches!(e, Event::Rejected { .. })),
        "a shared place is a place to go"
    );
}

#[test]
fn what_is_no_line_is_refused_and_nothing_goes_out() {
    let mut client = Client::new(Recipe::new(1)).unwrap();
    client.plug(chat::plugin());
    client.link_opened();
    client.receive(&welcome(&client));
    client.drain_events();
    client.command_json(r#"{"type":"chat.say","scope":"everywhere","text":"hi"}"#);
    client.command_json(r#"{"type":"chat.shout","text":"hi"}"#);
    let events = client.drain_events();
    assert!(
        events.len() == 2 && events.iter().all(|e| matches!(e, Event::Rejected { .. })),
        "{events:?}"
    );
    assert!(sent(&mut client).is_empty());
}
