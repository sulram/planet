//! Two people see each other: the client's side of the link, driven by hand
//! made server frames, the way a shell drives it with a socket.

use client::{Client, Event, Input, Outbound, PeerInfo, Recipe, SessionStatus};
use protocol::{Message, client_message, server_message};

fn server(message: server_message::Message) -> Vec<u8> {
    protocol::encode(&protocol::ServerMessage {
        message: Some(message),
    })
}

fn recipe_of(client: &Client) -> protocol::Recipe {
    let recipe = client.recipe();
    protocol::Recipe {
        seed: worldgen::format_seed(recipe.seed),
        generator_version: recipe.generator_version,
        params_json: serde_json::to_string(&recipe.params).unwrap(),
    }
}

fn stance(u: f32) -> protocol::Stance {
    protocol::Stance {
        sector: 0,
        u,
        v: 30_000.0,
        height_m: 5.0,
        facing_z: -1.0,
        gait: protocol::Gait::Walk.into(),
        speed_mps: 1.5,
        ..Default::default()
    }
}

fn peer(session: u32, name: &str) -> protocol::Peer {
    protocol::Peer {
        session,
        name: name.into(),
        visitor: name.is_empty(),
        avatar: String::new(),
        stance: Some(stance(30_000.0)),
    }
}

fn welcome(client: &Client, session: u32, peers: Vec<protocol::Peer>) -> Vec<u8> {
    server(server_message::Message::Welcome(protocol::Welcome {
        session,
        recipe: Some(recipe_of(client)),
        peers,
    }))
}

fn sent(client: &mut Client) -> Vec<client_message::Message> {
    client
        .drain_outbound()
        .into_iter()
        .filter_map(|out| match out {
            Outbound::Frame(frame) => protocol::ClientMessage::decode(&frame[..]).unwrap().message,
            Outbound::Close => None,
        })
        .collect()
}

fn run(client: &mut Client, seconds: f64) -> Vec<Event> {
    let mut events = Vec::new();
    for _ in 0..(seconds * 60.0) as u32 {
        client.update(1.0 / 60.0, &mut Input::default());
        events.extend(client.drain_events());
    }
    events
}

#[test]
fn hello_goes_out_and_welcome_brings_the_peers() {
    let mut client = Client::new(Recipe::new(1)).unwrap();
    client.drain_events();
    client.link_opened();
    let hello = sent(&mut client);
    assert!(
        matches!(&hello[..], [client_message::Message::Hello(h)] if h.protocol == protocol::PROTOCOL),
        "{hello:?}"
    );

    client.receive(&welcome(&client, 7, vec![peer(3, "Ada"), peer(5, "")]));
    let events = client.drain_events();
    assert!(events.contains(&Event::Session {
        status: SessionStatus::Online,
        session: Some(7)
    }));
    assert!(events.contains(&Event::Peers {
        peers: vec![
            PeerInfo {
                session: 3,
                name: "Ada".into(),
                visitor: false
            },
            PeerInfo {
                session: 5,
                name: String::new(),
                visitor: true
            },
        ]
    }));

    // Three bodies, the player's and two peers, all boxes: no file has landed.
    let frame = client.settled_frame();
    assert_eq!(frame.boxes.len(), 3 * 6, "{}", frame.boxes.len());
}

#[test]
fn a_peer_moves_between_the_stances_it_sent() {
    let mut client = Client::new(Recipe::new(1)).unwrap();
    client.link_opened();
    client.receive(&welcome(&client, 1, vec![peer(2, "Ada")]));
    run(&mut client, 0.5);
    let before = client.settled_frame().boxes[6].transform.translation;

    client.receive(&server(server_message::Message::Stances(
        protocol::Stances {
            moved: vec![protocol::Moved {
                session: 2,
                stance: Some(stance(30_010.0)),
            }],
        },
    )));
    run(&mut client, 0.5);
    let after = client.settled_frame().boxes[6].transform.translation;
    let moved = (after - before).length();
    // Ten blocks of half a metre, give or take the tangent warp.
    assert!((4.0..6.5).contains(&moved), "{moved}");

    // The player's own stance echoed back moves nobody.
    client.receive(&server(server_message::Message::Stances(
        protocol::Stances {
            moved: vec![protocol::Moved {
                session: 1,
                stance: Some(stance(0.0)),
            }],
        },
    )));
    run(&mut client, 0.5);
    assert_eq!(client.settled_frame().boxes.len(), 12);
}

#[test]
fn leaving_and_losing_the_link_take_the_bodies_away() {
    let mut client = Client::new(Recipe::new(1)).unwrap();
    client.link_opened();
    client.receive(&welcome(&client, 1, vec![peer(2, "Ada"), peer(3, "Bo")]));
    client.receive(&server(server_message::Message::Left(protocol::Left {
        session: 2,
    })));
    let events = client.drain_events();
    assert!(matches!(
        events.last(),
        Some(Event::Peers { peers }) if peers.len() == 1 && peers[0].session == 3
    ));
    assert_eq!(client.settled_frame().boxes.len(), 12);

    client.link_closed();
    let events = client.drain_events();
    assert!(events.contains(&Event::Session {
        status: SessionStatus::Offline,
        session: None
    }));
    assert!(events.contains(&Event::Peers { peers: vec![] }));
    assert_eq!(client.settled_frame().boxes.len(), 6);
}

#[test]
fn the_stance_goes_out_when_it_changes_and_as_a_heartbeat() {
    let mut client = Client::new(Recipe::new(1)).unwrap();
    client.link_opened();
    client.receive(&welcome(&client, 1, vec![]));
    sent(&mut client);

    // Standing still: one stance, then the heartbeat every two seconds.
    run(&mut client, 3.0);
    let stances = sent(&mut client)
        .into_iter()
        .filter(|m| matches!(m, client_message::Message::Stance(_)))
        .count();
    assert_eq!(stances, 2, "one at once, one two seconds later");

    // Walking: a stance about fifteen times a second.
    let mut input = Input::default();
    input.key(client::Key::Forward, true);
    for _ in 0..60 {
        client.update(1.0 / 60.0, &mut input);
    }
    let stances = sent(&mut client).len();
    assert!((10..=16).contains(&stances), "{stances}");
}

#[test]
fn another_recipe_is_another_world() {
    let mut client = Client::new(Recipe::new(1)).unwrap();
    client.link_opened();
    let mut recipe = recipe_of(&client);
    recipe.seed = worldgen::format_seed(2);
    client.receive(&server(server_message::Message::Welcome(
        protocol::Welcome {
            session: 1,
            recipe: Some(recipe),
            peers: vec![],
        },
    )));
    assert!(client.drain_outbound().contains(&Outbound::Close));
    assert!(client.drain_events().contains(&Event::Session {
        status: SessionStatus::Offline,
        session: None
    }));
}

#[test]
fn silence_ends_the_link() {
    let mut client = Client::new(Recipe::new(1)).unwrap();
    client.link_opened();
    client.receive(&welcome(&client, 1, vec![]));
    run(&mut client, 11.0);
    assert!(client.drain_outbound().contains(&Outbound::Close));
}
