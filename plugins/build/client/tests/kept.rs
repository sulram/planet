//! What is built is the world's: clients in one world, each with building
//! plugged in, and between them the server's own module, run here as the
//! server runs it. A change one makes is seen by who is near, kept for who
//! arrives, and taken back by the one who made it.

use std::sync::{Mutex, MutexGuard};

use client::{Client, Input, Outbound, Recipe, Seat};
use prost::Message;
use protocol::module::{Call, Op, Reply, Start, Who, call, reply};
use protocol::{client_message, server_message};
use topology::SurfacePoint;
use voxel::{Gesture, Span};

/// One session of the world: its client, and what the world holds of it.
struct Here {
    session: u32,
    level: protocol::Level,
    client: Client,
    stance: Option<protocol::Stance>,
}

/// A world of one module and the sessions in it: what the server's actor
/// does with a frame, less the socket.
struct World {
    recipe: protocol::Recipe,
    here: Vec<Here>,
    /// The module is one per process, as on the server: one world at a time
    /// holds it.
    _alone: MutexGuard<'static, ()>,
}

static MODULE: Mutex<()> = Mutex::new(());

fn serve(call: call::Call) -> Vec<reply::Reply> {
    let said = module::serve(&Call { call: Some(call) }.encode_to_vec());
    said.iter()
        .filter_map(|reply| Reply::decode(reply.as_slice()).ok()?.reply)
        .collect()
}

fn frame(message: server_message::Message) -> Vec<u8> {
    protocol::encode(&protocol::ServerMessage {
        message: Some(message),
    })
}

impl World {
    fn found() -> World {
        let alone = MODULE
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let recipe = Recipe::new(1);
        let recipe = protocol::Recipe {
            seed: worldgen::format_seed(recipe.seed),
            generator_version: recipe.generator_version,
            params_json: serde_json::to_string(&recipe.params).unwrap(),
        };
        serve(call::Call::Start(Start {
            recipe: Some(recipe.clone()),
        }));
        World {
            recipe,
            here: Vec::new(),
            _alone: alone,
        }
    }

    /// A client enters at a level and stands at a place. Its index here.
    fn enter(&mut self, level: protocol::Level, place: &str) -> usize {
        let session = self.here.len() as u32 + 1;
        let mut client = Client::new(Recipe::new(1)).unwrap();
        client.plug(build_client::plugin());
        client.go_to(place).expect("a place to stand");
        client.link_opened();
        client.receive(&frame(server_message::Message::Welcome(
            protocol::Welcome {
                session,
                recipe: Some(self.recipe.clone()),
                peers: vec![],
                level: level.into(),
                plugins: vec![protocol::Plugin {
                    name: build_client::NAME.into(),
                    version: build_client::VERSION,
                }],
            },
        )));
        self.here.push(Here {
            session,
            level,
            client,
            stance: None,
        });
        self.here.len() - 1
    }

    /// Carries what every client said to the module and what the module
    /// said back, until nobody has more to say.
    fn carry(&mut self) {
        loop {
            let mut said = Vec::new();
            for at in 0..self.here.len() {
                for out in self.here[at].client.drain_outbound() {
                    if let Outbound::Frame(frame) = out {
                        said.push((at, frame));
                    }
                }
            }
            if said.is_empty() {
                return;
            }
            for (at, frame) in said {
                let message = protocol::ClientMessage::decode(&frame[..]).unwrap();
                match message.message {
                    Some(client_message::Message::Stance(stance)) => {
                        self.here[at].stance = Some(stance);
                    }
                    Some(client_message::Message::Envelope(envelope)) => self.op(at, envelope),
                    _ => {}
                }
            }
        }
    }

    /// One op on its way to its owner: the level is asked first, as the
    /// server's actor asks it, and the asker is answered.
    fn op(&mut self, at: usize, envelope: protocol::Envelope) {
        let builder = ["open", "change", "close", "take_back", "put_back"];
        let builder = builder.contains(&envelope.kind.as_str());
        let may = !builder || self.here[at].level >= protocol::Level::Builder;
        let mut code = match may {
            true => String::new(),
            false => "level".to_owned(),
        };
        if may {
            let here = self.here.iter().map(|here| Who {
                session: here.session,
                level: here.level.into(),
                stance: here.stance,
                ..Default::default()
            });
            let said = serve(call::Call::Op(Op {
                plugin: envelope.plugin.clone(),
                kind: envelope.kind.clone(),
                payload: envelope.payload,
                session: self.here[at].session,
                now_ms: 1,
                here: here.collect(),
            }));
            for said in said {
                match said {
                    reply::Reply::Answered(answered) => code = answered.code,
                    reply::Reply::Tell(tell) => {
                        let event = frame(server_message::Message::Envelope(protocol::Envelope {
                            plugin: tell.plugin,
                            kind: tell.kind,
                            payload: tell.payload,
                            id: 0,
                        }));
                        for here in &mut self.here {
                            if tell.to.contains(&here.session) {
                                here.client.receive(&event);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        if envelope.id != 0 {
            let answer = protocol::Answer {
                id: envelope.id,
                code,
            };
            self.here[at]
                .client
                .receive(&frame(server_message::Message::Answer(answer)));
        }
    }

    /// Every client lives `seconds` of frames, and the world carries what
    /// they say after each.
    fn live(&mut self, seconds: f64) {
        for _ in 0..(seconds * 60.0) as u32 {
            for here in &mut self.here {
                here.client.update(1.0 / 60.0, &mut Input::default());
                here.client.drain_volume_changes();
                here.client.drain_terrain_changes();
            }
            self.carry();
        }
    }

    fn client(&mut self, at: usize) -> &mut Client {
        &mut self.here[at].client
    }
}

/// The solid cells two clients hold of the volume over a column, when both
/// hold it: the same, cell for cell.
fn same(a: &Client, b: &Client, point: SurfacePoint) -> Option<usize> {
    let (held, also) = (
        a.cells().bounds_over(point.sector.into(), point)?,
        b.cells().bounds_over(point.sector.into(), point)?,
    );
    assert_eq!(held, also);
    let mut solid = 0;
    for at in held.cells() {
        let cell = a.cells().cell(point.sector, at);
        assert_eq!(cell, b.cells().cell(point.sector, at), "{at:?}");
        solid += usize::from(!cell.is_air());
    }
    Some(solid)
}

#[test]
fn what_is_built_is_the_worlds() {
    let mut world = World::found();
    let (a, b) = (
        world.enter(protocol::Level::Builder, "4-77AEYRG"),
        world.enter(protocol::Level::Builder, "4-77AEYRG"),
    );
    // A visitor a long walk away, on the same sector.
    let far = world.enter(protocol::Level::Anonymous, "4-77AEYRG");
    let point = world.client(a).host("build").feet().point;
    let away = SurfacePoint::new(point.sector, point.u + 4000.0, point.v);
    world.client(far).teleport(away);
    world.live(0.5);

    // One lays a platform. The other, beside it, holds the same cells; the
    // visitor far away holds none of it.
    world
        .client(a)
        .command_json(r#"{"type":"build.platform","side":16}"#);
    world
        .client(a)
        .command_json(r#"{"type":"build.lay","base":"deck"}"#);
    world.live(0.5);
    assert_eq!(world.client(a).cells().history(), (true, false));
    let slab = same(&world.here[a].client, &world.here[b].client, point).expect("both hold it");
    assert!(slab >= 256, "{slab}");
    assert!(!world.client(far).cells().covers(point.sector.into(), point));
    assert_eq!(world.client(b).cells().history(), (false, false));

    // The visitor walks over, and is shown what stands there.
    world.client(far).teleport(point);
    world.live(1.5);
    assert_eq!(
        same(&world.here[a].client, &world.here[far].client, point),
        Some(slab)
    );

    // Both builders fill the same box at once, each in a paint of its own.
    // The world takes one first, and both end with what the world holds.
    let held = world
        .client(a)
        .cells()
        .bounds_over(point.sector.into(), point)
        .unwrap();
    let top = (held.min[2]..=held.max[2])
        .rev()
        .find(|&z| {
            !world
                .client(a)
                .cells()
                .cell(point.sector, [point.u as i32, point.v as i32, z])
                .is_air()
        })
        .expect("the slab");
    let (x, y) = (point.u as i32, point.v as i32);
    let span = Span::between([x, y, top + 1], [x + 2, y, top + 1]);
    let seat = Seat::Sector(point.sector);
    for (who, paint) in [(a, 1), (b, 2)] {
        let made = world
            .client(who)
            .host("build")
            .apply(seat, &[Gesture::Create { span, paint }]);
        assert_eq!(made, Ok(true));
    }
    assert_eq!(
        world.client(b).cells().cell(point.sector, span.min).paint(),
        Some(2)
    );
    world.live(0.2);
    for who in [a, b, far] {
        let cell = world.client(who).cells().cell(point.sector, span.min);
        assert_eq!(cell.paint(), Some(1), "client {who}");
    }
    assert_eq!(
        same(&world.here[a].client, &world.here[b].client, point),
        Some(slab + 3)
    );

    // The one who made it takes it back, and it is gone for everyone.
    world.client(a).host("build").take_back().unwrap();
    world.live(0.2);
    for who in [a, b, far] {
        assert!(
            world
                .client(who)
                .cells()
                .cell(point.sector, span.min)
                .is_air(),
            "client {who}"
        );
    }
    assert_eq!(world.client(a).cells().history(), (true, true));
    world.client(a).host("build").put_back().unwrap();
    world.live(0.2);
    assert_eq!(
        same(&world.here[a].client, &world.here[far].client, point),
        Some(slab + 3)
    );
    assert_eq!(world.client(a).cells().history(), (true, false));

    // A visitor changes nothing, here or in the world.
    let refused = world
        .client(far)
        .host("build")
        .apply(seat, &[Gesture::Delete { span }]);
    assert_eq!(refused, Err(client::Refusal::Level));

    // One closes the volume, and it is gone for everyone near, cells and
    // all. It is a change of theirs: taken back, the volume stands again for
    // everyone as it was, and put back, it is gone again.
    world.client(a).command_json(r#"{"type":"build.close"}"#);
    assert!(!world.client(a).cells().covers(point.sector.into(), point));
    world.live(0.2);
    for who in [a, b, far] {
        assert!(
            !world.client(who).cells().covers(point.sector.into(), point),
            "client {who}"
        );
    }
    assert_eq!(world.client(a).cells().history(), (true, false));
    world.client(a).command_json(r#"{"type":"build.undo"}"#);
    world.live(0.2);
    for who in [b, far] {
        assert_eq!(
            same(&world.here[a].client, &world.here[who].client, point),
            Some(slab + 3),
            "client {who}"
        );
    }
    world.client(a).command_json(r#"{"type":"build.redo"}"#);
    world.live(0.2);
    assert!(!world.client(b).cells().covers(point.sector.into(), point));
    world.client(a).command_json(r#"{"type":"build.undo"}"#);
    world.live(0.2);
    assert_eq!(
        same(&world.here[a].client, &world.here[b].client, point),
        Some(slab + 3)
    );
    // A visitor closes none.
    world.client(far).command_json(r#"{"type":"build.close"}"#);
    world.live(0.2);
    assert!(world.client(a).cells().covers(point.sector.into(), point));

    // Who walks away lets the volume go, and a link that drops takes what
    // the world kept with it.
    world.client(b).teleport(away);
    world.live(1.5);
    assert!(!world.client(b).cells().covers(point.sector.into(), point));
    world.client(a).link_closed();
    assert!(!world.client(a).cells().covers(point.sector.into(), point));
}

#[test]
fn a_volume_far_off_is_held_as_it_is_seen_from_afar() {
    let mut world = World::found();
    let a = world.enter(protocol::Level::Builder, "4-77AEYRG");
    let far = world.enter(protocol::Level::Anonymous, "4-77AEYRG");
    // Three plots along from where the other test of this module builds:
    // the module is one per process, and keeps what that one left.
    let start = world.client(a).host("build").feet().point;
    let point = SurfacePoint::new(start.sector, start.u + 192.0, start.v);
    world.client(a).teleport(point);
    let seat = Seat::Sector(point.sector);
    let along = |blocks: f64| SurfacePoint::new(point.sector, point.u + blocks, point.v);
    // A kilometre off, along the sector.
    world.client(far).teleport(along(2000.0));
    world.live(0.5);
    world
        .client(a)
        .command_json(r#"{"type":"build.platform","side":16}"#);
    world
        .client(a)
        .command_json(r#"{"type":"build.lay","base":"deck"}"#);
    world.live(1.5);

    // The visitor holds none of its cells, and draws it as it is seen from
    // afar.
    assert!(!world.client(far).cells().covers(seat, point));
    assert_eq!(world.client(far).cells().afar_over(seat, point), Some(4));
    assert!(!world.client(far).settled_frame().volumes.is_empty());

    // Walked over, it is held whole, and once it is drawn whole what was
    // drawn afar goes.
    world.client(far).teleport(point);
    world.live(1.5);
    assert_eq!(
        same(&world.here[a].client, &world.here[far].client, point),
        same(&world.here[a].client, &world.here[a].client, point)
    );
    assert_eq!(world.client(far).cells().afar_over(seat, point), None);

    // Walked off past holding distance, it is held afar again, made from
    // what was held of it, and nothing is asked of the world for it.
    world.client(far).teleport(along(1000.0));
    world.live(1.5);
    assert!(!world.client(far).cells().covers(seat, point));
    assert_eq!(world.client(far).cells().afar_over(seat, point), Some(4));

    // Three kilometres off, it is held a step further, made from the copy
    // held afar, and still drawn.
    world.client(far).teleport(along(6000.0));
    world.live(1.5);
    assert_eq!(world.client(far).cells().afar_over(seat, point), Some(16));
    assert!(!world.client(far).settled_frame().volumes.is_empty());

    // Past the last step, it is let go.
    let beyond = if point.u > 32768.0 { -19000.0 } else { 19000.0 };
    world.client(far).teleport(along(beyond));
    world.live(1.5);
    assert_eq!(world.client(far).cells().afar_over(seat, point), None);
    assert!(world.client(far).settled_frame().volumes.is_empty());
}
