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
        name: None,
        muted: false,
        volume: 0.0,
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
        millis: 0,
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
        name: None,
        muted: false,
        volume: 0.0,
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
        name: None,
        muted: false,
        volume: 0.0,
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
        .place_at(charted[0].seat, &charted[0].part.pose);
    assert!(client.go_to(&place).is_ok(), "{place} takes a body there");
    // The place says which way to face on arrival: the way the star's
    // point toward `+y` points, level; arrived, the body faces it.
    let (_, look) = place.split_once(',').expect("a bearing and a pitch");
    let (bearing, pitch) = look.split_once(',').unwrap();
    let bearing: f64 = bearing.parse().unwrap();
    assert_eq!(pitch, "0", "{place}");
    let faced = client.here().bearing_deg.unwrap();
    assert!((faced - bearing).abs() < 1.0, "{place}: facing {faced}");
    assert!(client.host("test").take_charted().is_none(), "taken once");
}

/// A part on the moon has its place on the moon: the place the host says
/// for a landmark there takes the body to it, beside its star, wherever the
/// moon has moved to.
#[test]
fn a_landmark_on_the_moon_is_travelled_to_on_the_moon() {
    let mut client = Client::new(Recipe::new(1)).unwrap();
    client.rehearse(client::Level::Admin);
    let sector = topology::Sector::new(2).unwrap();
    let seat = Seat::on(protocol::Body::Moon, sector);
    let point = topology::SurfacePoint::new(sector, 3000.0, 5000.0);
    let ground = client.host("test").ground(seat, point);
    let pose = PartPose::upright([3000.0, 5000.0, ground], [2.0, 2.0, 0.16]);
    let part = Part {
        shape: Shape::Landmark(Name::new("Mare").unwrap()),
        pose,
        paint: 0,
        solid: false,
        shine: 0.0,
        reach_m: 0.0,
        name: None,
        muted: false,
        volume: 0.0,
    };
    client.host("test").place_part(seat, part).unwrap();
    let place = client.host("test").place_at(seat, &pose);
    assert!(place.starts_with('m'), "{place} is on the moon");
    client.go_to(&place).unwrap();
    let feet = client.host("test").feet();
    assert_eq!(feet.seat.body(), protocol::Body::Moon);
    assert!(
        (feet.point.u - 3000.0).abs() < 2.0 && (feet.point.v - 5000.0).abs() < 2.0,
        "landed at {:?}, the star at (3000, 5000)",
        feet.point
    );
}

/// A body that falls into a portal before the place it leads to is known
/// is held there, the picture drawn in, the body where it crossed; the
/// place said late, it goes on and lands there. Let out instead, it comes
/// out where it fell in.
#[test]
fn a_body_is_held_in_a_portal_until_its_place_is_said() {
    let mut client = welcomed();
    client.set_view([1600, 900]);
    let feet = client.host("test").feet();
    let seat = Seat::Sector(feet.point.sector);
    let ground = feet.height_m / topology::BLOCK_M;
    let star = Part {
        shape: Shape::Landmark(Name::new("Far").unwrap()),
        pose: PartPose::upright(
            [feet.point.u, feet.point.v + 300.0, ground],
            [2.0, 2.0, 0.16],
        ),
        paint: 0,
        solid: false,
        shine: 0.0,
        reach_m: 0.0,
        name: None,
        muted: false,
        volume: 0.0,
    };
    let ring = Part {
        shape: Shape::Portal(client::Portal {
            to: None,
            rim: client::Portal::RIM,
        }),
        pose: PartPose::upright([feet.point.u, feet.point.v + 2.0, ground], [4.0, 0.1, 4.0]),
        ..star
    };
    let ring = client.host("test").place_part(seat, ring).unwrap();
    let mut input = client::Input::default();
    client.host("test").fall_into(ring);
    for _ in 0..90 {
        client.update(1.0 / 60.0, &mut input);
    }
    let frame = client.update(1.0 / 60.0, &mut input);
    let warp = frame.warp.expect("held in the portal");
    assert_eq!(warp.phase, 0.0, "drawn in, not let go");
    assert_eq!(
        client.host("test").feet().point,
        feet.point,
        "where it crossed"
    );
    // The place, said late: through, and landed by the star.
    let place = client.host("test").place_at(seat, &star.pose);
    client.host("test").go_through(&place, ring).unwrap();
    let mut frames = 0;
    while client.update(1.0 / 60.0, &mut input).warp.is_some() {
        frames += 1;
        assert!(frames < 60 * 10, "the passage ends");
    }
    let landed = client.host("test").feet().point;
    assert!(
        (landed.v - star.pose.at[1]).abs() < 2.0,
        "landed at {landed:?}, the star at {:?}",
        star.pose.at
    );
    // Held again and let out: it comes out where it fell in, in a moment.
    client.host("test").fall_into(ring);
    for _ in 0..30 {
        client.update(1.0 / 60.0, &mut input);
    }
    client.host("test").let_out();
    let mut frames = 0;
    while client.update(1.0 / 60.0, &mut input).warp.is_some() {
        frames += 1;
        assert!(frames < 60, "let out within a second");
    }
    assert_eq!(client.host("test").feet().point, landed, "where it fell in");
}

/// A portal bears the name of the star it leads to over its ring, as the
/// star bears its own, so where it leads is read before it is walked.
#[test]
fn a_portal_bears_the_name_of_the_star_it_leads_to() {
    let mut client = welcomed();
    client.set_view([1600, 900]);
    let feet = client.host("test").feet();
    let seat = Seat::Sector(feet.point.sector);
    let star = Part {
        shape: Shape::Landmark(Name::new("Far").unwrap()),
        pose: PartPose::upright(
            [
                feet.point.u,
                feet.point.v + 8000.0,
                feet.height_m / topology::BLOCK_M,
            ],
            [2.0, 2.0, 0.16],
        ),
        paint: 0,
        solid: false,
        shine: 0.0,
        reach_m: 0.0,
        name: None,
        muted: false,
        volume: 0.0,
    };
    let star = client.host("test").place_part(seat, star).unwrap();
    let ring = Part {
        shape: Shape::Portal(client::Portal {
            to: Some(star),
            rim: client::Portal::RIM,
        }),
        pose: PartPose::upright(
            [
                feet.point.u,
                feet.point.v + 12.0,
                feet.height_m / topology::BLOCK_M,
            ],
            [4.0, 0.1, 4.0],
        ),
        paint: 0,
        solid: false,
        shine: 0.0,
        reach_m: 0.0,
        name: None,
        muted: false,
        volume: 0.0,
    };
    let ring = client.host("test").place_part(seat, ring).unwrap();
    let mut input = client::Input::default();
    client.update(1.0 / 60.0, &mut input);
    let labels: Vec<client::Label> = client
        .drain_events()
        .into_iter()
        .filter_map(|event| match event {
            Event::Labels { labels } => Some(labels),
            _ => None,
        })
        .flatten()
        .collect();
    let over = labels
        .iter()
        .find(|label| label.id == ring.0)
        .expect("the ring in view bears a label");
    assert_eq!(over.text, "Far");
    assert!(
        labels.iter().all(|label| label.id != star.0),
        "the star itself is too far for its own"
    );
}

/// A portal seen through puts the star it leads to in the cells' look as a
/// second eye, so what is built there comes and shows through the pane.
#[test]
fn a_portal_seen_through_looks_at_the_cells_by_its_star() {
    let mut client = welcomed();
    client.set_view([1600, 900]);
    let feet = client.host("test").feet();
    let seat = Seat::Sector(feet.point.sector);
    let star_at = [
        feet.point.u,
        feet.point.v + 8000.0,
        feet.height_m / topology::BLOCK_M,
    ];
    let star = Part {
        shape: Shape::Landmark(Name::new("Far").unwrap()),
        pose: PartPose::upright(star_at, [2.0, 2.0, 0.16]),
        paint: 0,
        solid: false,
        shine: 0.0,
        reach_m: 0.0,
        name: None,
        muted: false,
        volume: 0.0,
    };
    let star = client.host("test").place_part(seat, star).unwrap();
    let ring = Part {
        shape: Shape::Portal(client::Portal {
            to: Some(star),
            rim: client::Portal::RIM,
        }),
        pose: PartPose::upright(
            [
                feet.point.u,
                feet.point.v + 6.0,
                feet.height_m / topology::BLOCK_M,
            ],
            [4.0, 0.1, 4.0],
        ),
        paint: 0,
        solid: false,
        shine: 0.0,
        reach_m: 0.0,
        name: None,
        muted: false,
        volume: 0.0,
    };
    client.host("test").place_part(seat, ring).unwrap();
    client.drain_outbound();
    let mut input = client::Input::default();
    for _ in 0..70 {
        client.update(1.0 / 60.0, &mut input);
    }
    let looks: Vec<protocol::cells::Look> = client
        .drain_outbound()
        .into_iter()
        .filter_map(|out| match out {
            Outbound::Frame(frame) => {
                match protocol::ClientMessage::decode(&frame[..]).unwrap().message {
                    Some(client_message::Message::Envelope(envelope))
                        if envelope.plugin == protocol::cells::OWNER
                            && envelope.kind == protocol::cells::LOOK =>
                    {
                        protocol::cells::Look::decode(envelope.payload.as_slice()).ok()
                    }
                    _ => None,
                }
            }
            _ => None,
        })
        .collect();
    let eye = looks
        .iter()
        .find_map(|look| look.eye.as_ref())
        .expect("the look carries the eye past the portal");
    assert!(
        (f64::from(eye.v) - star_at[1]).abs() < 2.0 && (f64::from(eye.u) - star_at[0]).abs() < 2.0,
        "the eye stands by the star: {eye:?}"
    );
}
