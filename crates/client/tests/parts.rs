//! The parts in a world: a part placed here is asked of the world, which
//! names it; what is asked of it before then waits for the name; what
//! others place arrives; a place refused is taken back.

use client::{
    CUBE, Client, ContentId, ImageKind, Media, Outbound, Part, PartPose, Recipe, Seat, Shape,
};
use protocol::parts as wire;
use protocol::{Message, client_message, server_message};
use topology::Sector;

fn server(message: server_message::Message) -> Vec<u8> {
    protocol::encode(&protocol::ServerMessage {
        message: Some(message),
    })
}

fn welcomed() -> Client {
    let mut client = Client::new(Recipe::new(1)).unwrap();
    let recipe = client.recipe();
    client.receive(&server(server_message::Message::Welcome(
        protocol::Welcome {
            session: 1,
            recipe: Some(protocol::Recipe {
                seed: worldgen::format_seed(recipe.seed),
                generator_version: recipe.generator_version,
                params_json: serde_json::to_string(&recipe.params).unwrap(),
            }),
            peers: vec![],
            level: protocol::Level::Builder.into(),
            plugins: vec![],
        },
    )));
    client.drain_events();
    client.drain_outbound();
    client
}

fn asked(client: &mut Client) -> Vec<protocol::Envelope> {
    client
        .drain_outbound()
        .into_iter()
        .filter_map(|out| match out {
            Outbound::Frame(frame) => {
                match protocol::ClientMessage::decode(&frame[..]).unwrap().message {
                    Some(client_message::Message::Envelope(envelope))
                        if envelope.plugin == wire::OWNER =>
                    {
                        Some(envelope)
                    }
                    _ => None,
                }
            }
            Outbound::Close => None,
        })
        .collect()
}

fn said(kind: &str, payload: Vec<u8>) -> Vec<u8> {
    server(server_message::Message::Envelope(protocol::Envelope {
        plugin: wire::OWNER.into(),
        kind: kind.into(),
        payload,
        id: 0,
    }))
}

fn answer(id: u32, code: &str) -> Vec<u8> {
    server(server_message::Message::Answer(protocol::Answer {
        id,
        code: code.into(),
    }))
}

fn cube(u: f64) -> (Seat, Part) {
    let seat = Seat::Sector(Sector::new(4).unwrap());
    let part = Part {
        shape: Shape::Cube,
        pose: PartPose::upright([u, 20000.0, 10.0], CUBE),
        paint: 5,
        solid: true,
    };
    (seat, part)
}

fn record(id: u64, version: u64, seat: Seat, part: &Part) -> wire::Record {
    wire::Record {
        id,
        part: Some(seat::part_wire(seat, part)),
        version,
    }
}

#[test]
fn a_part_placed_is_asked_of_the_world_and_set_under_the_name_it_gives() {
    let mut client = welcomed();
    let (seat, part) = cube(20000.0);
    let local = client.host("test").place_part(seat, part).unwrap();
    let place = asked(&mut client);
    assert_eq!(place.len(), 1);
    assert_eq!(place[0].kind, wire::PLACE);
    // Set twice before the world named it: nothing goes out yet, and the
    // part here is as the last set left it.
    let mut moved = part;
    for h in [11.0, 12.0] {
        moved.pose.at[2] = h;
        client.host("test").set_part(local, seat, moved).unwrap();
    }
    assert!(asked(&mut client).is_empty());
    assert_eq!(client.parts().get(local).unwrap().1.pose.at[2], 12.0);
    // The world names it, and the cadence comes: one set goes out under
    // that name, the latest, and the part keeps the name it has here.
    let placed = wire::Placed {
        session: 1,
        record: Some(record(77, 0, seat, &part)),
    };
    client.receive(&said(wire::PLACED, placed.encode_to_vec()));
    client.receive(&answer(place[0].id, ""));
    let mut input = client::Input::default();
    for _ in 0..8 {
        client.update(1.0 / 60.0, &mut input);
    }
    let set: Vec<_> = asked(&mut client)
        .into_iter()
        .filter(|ask| ask.kind == wire::SET)
        .collect();
    assert_eq!(set.len(), 1, "one set, the latest, beside the look");
    let set = wire::Set::decode(&set[0].payload[..]).unwrap();
    assert_eq!(set.id, 77);
    assert_eq!(set.part.unwrap().pose.unwrap().h, 12.0);
    assert!(client.parts().get(local).is_some());
    assert_eq!(client.parts().all().count(), 1);
    // Removed here, removed there.
    client.host("test").remove_part(local).unwrap();
    let remove = asked(&mut client);
    assert_eq!(wire::Remove::decode(&remove[0].payload[..]).unwrap().id, 77);
    assert_eq!(client.parts().all().count(), 0);
}

#[test]
fn what_others_place_arrives_and_what_is_refused_is_taken_back() {
    let mut client = welcomed();
    let (seat, part) = cube(20010.0);
    // Someone else's, placed and then seen again stale, then gone.
    let placed = wire::Placed {
        session: 2,
        record: Some(record(5, 0, seat, &part)),
    };
    client.receive(&said(wire::PLACED, placed.encode_to_vec()));
    assert_eq!(client.parts().all().count(), 1);
    let mut repainted = part;
    repainted.paint = 9;
    let seen = wire::Seen {
        records: vec![record(5, 1, seat, &repainted)],
        gone: vec![],
    };
    client.receive(&said(wire::SEEN, seen.encode_to_vec()));
    let held: Vec<_> = client.parts().all().collect();
    assert_eq!((held.len(), held[0].2.paint), (1, 9));
    let removed = wire::Removed { session: 2, id: 5 };
    client.receive(&said(wire::REMOVED, removed.encode_to_vec()));
    assert_eq!(client.parts().all().count(), 0);
    // A place of this session's refused comes back off.
    let local = client.host("test").place_part(seat, part).unwrap();
    let place = asked(&mut client);
    assert!(client.parts().get(local).is_some());
    client.receive(&answer(place[0].id, "pose"));
    assert!(client.parts().get(local).is_none());
    // Every second, what is held is asked about.
    let mut input = client::Input::default();
    for _ in 0..70 {
        client.update(1.0 / 60.0, &mut input);
    }
    assert!(asked(&mut client).iter().any(|ask| ask.kind == wire::LOOK));
}

/// A picture placed by someone else arrives as a picture: drawn as a quad
/// of its aspect, asked for at the level it is seen at, and no model.
#[test]
fn a_picture_another_placed_arrives_and_is_drawn_as_one() {
    let mut client = welcomed();
    client.set_view([1600, 900]);
    let seat = Seat::Sector(Sector::new(4).unwrap());
    let media = Media {
        width: 3000,
        height: 2000,
        thumb: ContentId([1; 32]),
        low: ContentId([2; 32]),
        full: ContentId([3; 32]),
        kind: ImageKind::Jpg,
    };
    let feet = client.host("test").feet();
    let part = Part {
        shape: Shape::Image(media),
        pose: PartPose::upright(
            [
                feet.point.u,
                feet.point.v + 4.0,
                feet.height_m / topology::BLOCK_M,
            ],
            [3.0, 0.1, 2.0],
        ),
        paint: 0,
        solid: false,
    };
    let placed = wire::Placed {
        session: 2,
        record: Some(record(9, 0, seat, &part)),
    };
    client.receive(&said(wire::PLACED, placed.encode_to_vec()));
    let frame = client.settled_frame();
    assert_eq!(frame.pictures.len(), 1);
    assert!(frame.models.is_empty(), "a picture is no model");
    let quad = frame.pictures[0].transform.matrix3;
    assert!((quad.x_axis.length() / quad.y_axis.length() - 1.5).abs() < 1e-6);
    assert!(
        client
            .drain_asset_requests()
            .iter()
            .any(
                |request| request.path == media.name(client::PictureLevel::Thumb)
                    || request.path == media.name(client::PictureLevel::Low)
            ),
        "a level of it is asked for"
    );
}
