//! The parts in a world: a part placed here is asked of the world, which
//! names it; what is asked of it before then waits for the name; what
//! others place arrives; a place refused is taken back.

use client::{
    CUBE, Client, ContentId, Event, ImageKind, Media, Name, Outbound, Part, PartPose, Recipe, Seat,
    Shape,
};
use protocol::parts as wire;
use protocol::{Message, client_message, server_message};

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
            clock_ms: 0,
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

/// A cube a metre a side, standing where the client's own body is, a few
/// blocks along from it: near enough to be held, and placed by it.
fn cube(client: &mut Client, along: f64) -> (Seat, Part) {
    let feet = client.host("test").feet();
    let part = Part {
        shape: Shape::Cube,
        pose: PartPose::upright(
            [
                feet.point.u + along,
                feet.point.v,
                feet.height_m / topology::BLOCK_M,
            ],
            CUBE,
        ),
        paint: 5,
        solid: true,
        shine: 0.0,
        reach_m: 0.0,
    };
    (feet.seat, part)
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
    let (seat, part) = cube(&mut client, 4.0);
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

/// With no world, one history takes back a stroke of cells and a part
/// placed in the order they were made, and puts them back (DECISIONS 115).
#[test]
fn one_history_takes_back_cells_and_parts_in_order() {
    use client::{Key, Level};
    let mut client = Client::new(Recipe::new(1)).unwrap();
    client.rehearse(Level::Admin);
    client.set_view([1600, 900]);
    // A part, then a cell in a volume opened under the feet, then the part
    // set twice as a drag does.
    let (seat, part) = cube(&mut client, 4.0);
    let standing = part.pose.at[2];
    let id = client.host("test").place_part(seat, part).unwrap();
    let feet = client.host("test").feet();
    client.host("test").open(feet.seat, feet.point).unwrap();
    let room = client.host("test").room(feet.seat, feet.point).unwrap();
    let cell = [room.min[0] + 2, room.min[1] + 2, room.min[2] + 4];
    let gesture = voxel::Gesture::Create {
        span: voxel::Span::cell(cell),
        paint: 1,
    };
    assert!(client.host("test").apply(feet.seat, &[gesture]).unwrap());
    let solid = |client: &Client| !client.cells().cell(feet.seat, cell).is_air();
    assert!(solid(&client));
    let mut moved = part;
    for h in [11.0, 12.0] {
        moved.pose.at[2] = h;
        client.host("test").set_part(id, seat, moved).unwrap();
    }
    assert_eq!(client.parts().get(id).unwrap().1.pose.at[2], 12.0);
    assert_eq!(client.history().state(), (true, false));
    let key = |client: &mut Client, key: Key| {
        let mut input = client::Input::default();
        input.key(key, true);
        client.update(1.0 / 60.0, &mut input);
    };
    // Back: the two sets as one, then the cell, then the part.
    key(&mut client, Key::Undo);
    assert_eq!(client.parts().get(id).unwrap().1.pose.at[2], standing);
    key(&mut client, Key::Undo);
    assert!(!solid(&client));
    key(&mut client, Key::Undo);
    assert!(client.parts().get(id).is_none());
    assert_eq!(client.history().state(), (false, true));
    // Forward again, in order.
    key(&mut client, Key::Redo);
    assert!(client.parts().get(id).is_some());
    key(&mut client, Key::Redo);
    assert!(solid(&client));
    key(&mut client, Key::Redo);
    assert_eq!(client.parts().get(id).unwrap().1.pose.at[2], 12.0);
    assert_eq!(client.history().state(), (true, false));
}

#[test]
fn what_others_place_arrives_and_what_is_refused_is_taken_back() {
    let mut client = welcomed();
    let (seat, part) = cube(&mut client, 8.0);
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
    let feet = client.host("test").feet();
    // Beside the body, within reach of its eye.
    let seat = Seat::Sector(feet.point.sector);
    let media = Media {
        width: 3000,
        height: 2000,
        thumb: ContentId([1; 32]),
        low: ContentId([2; 32]),
        full: ContentId([3; 32]),
        kind: ImageKind::Jpg,
    };
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
        shine: 0.0,
        reach_m: 0.0,
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

/// A landmark placed by someone else arrives as a star: drawn as a solid
/// model whatever its paint, a lamp, its name hung over it when it is in
/// view, and said by the chart with the place that takes a body to it.
#[test]
fn a_landmark_another_placed_is_a_lit_star_with_its_name_over_it() {
    let mut client = welcomed();
    client.set_view([1600, 900]);
    let feet = client.host("test").feet();
    let seat = Seat::Sector(feet.point.sector);
    let name = Name::new("Praça da Sé").unwrap();
    let part = Part {
        shape: Shape::Landmark(name),
        pose: PartPose::upright(
            [
                feet.point.u,
                feet.point.v + 6.0,
                feet.height_m / topology::BLOCK_M,
            ],
            [2.0, 2.0, 0.16],
        ),
        // A glass paint: a landmark is solid to the eye all the same.
        paint: voxel::Paint {
            color: 3,
            finish: voxel::Finish::Glass,
            edge: voxel::Edge::None,
        }
        .byte(),
        solid: false,
        shine: 0.0,
        reach_m: 0.0,
    };
    let placed = wire::Placed {
        session: 2,
        record: Some(record(9, 0, seat, &part)),
    };
    client.receive(&said(wire::PLACED, placed.encode_to_vec()));
    let frame = client.settled_frame();
    assert_eq!(frame.models.len(), 1);
    assert_eq!(frame.models[0].pass, scene::ModelPass::Solid);
    assert_eq!(frame.lamps.len(), 1, "a landmark lights what is near");
    // Facing it, its name is on the screen; turned away, the labels empty
    // once and then say nothing.
    let mut input = client::Input::default();
    client.update(1.0 / 60.0, &mut input);
    let labels: Vec<Vec<client::Label>> = client
        .drain_events()
        .into_iter()
        .filter_map(|event| match event {
            Event::Labels { labels } => Some(labels),
            _ => None,
        })
        .collect();
    let seen = labels.iter().flatten().find(|label| label.id == 1);
    let seen = seen.expect("the star is in view of a body facing along v");
    assert_eq!(seen.text, "Praça da Sé");
    assert!((0.0..=1.0).contains(&seen.x) && (0.0..=1.0).contains(&seen.y));
    assert!(seen.distance_m < 10.0);
    // The chart, in a world, is asked of the world and its answer taken.
    client.host("test").chart();
    assert!(asked(&mut client).iter().any(|ask| ask.kind == wire::CHART));
    let charted = wire::Charted {
        records: vec![record(9, 0, seat, &part)],
    };
    client.receive(&said(wire::CHARTED, charted.encode_to_vec()));
    let charted = client.host("test").take_charted().unwrap();
    assert_eq!((charted.len(), charted[0].id), (1, 9));
    assert!(charted[0].away_m < 10.0);
    let place = client
        .host("test")
        .place_at(charted[0].seat, charted[0].part.pose.at);
    assert!(client.go_to(&place).is_ok(), "{place} takes a body there");
    assert!(client.host("test").take_charted().is_none(), "taken once");
}
