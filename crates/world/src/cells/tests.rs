//! The cells' system over a room a test holds: a store in memory, the
//! generated ground of the test world, and whoever a test puts in it.

use std::collections::BTreeMap;

use protocol::Stance;
use topology::BLOCK_M;
use worldgen::{Generator, Recipe};

use super::*;
use crate::{Measure, Moment};

/// A room that keeps what it is told and what it is given to keep, as the
/// server does once an op has been applied.
struct Stage {
    ground: Option<Generator>,
    measure: Measure,
    here: Vec<Who>,
    store: BTreeMap<Vec<u8>, Vec<u8>>,
    told: Vec<(String, Vec<u8>, Vec<u32>)>,
    refused: Option<String>,
}

impl Room for Stage {
    fn now(&self) -> Moment {
        Moment(0)
    }

    fn measure(&self) -> Measure {
        self.measure
    }

    fn sessions(&self) -> &[Who] {
        &self.here
    }

    fn tell(&mut self, kind: &str, payload: Vec<u8>, to: &[u32]) {
        self.told.push((kind.to_owned(), payload, to.to_vec()));
    }

    fn ground(&self) -> Option<&Generator> {
        self.ground.as_ref()
    }

    fn get(&mut self, key: &[u8]) -> Option<Vec<u8>> {
        self.store.get(key).cloned()
    }

    fn scan(&mut self, prefix: &[u8]) -> Vec<(Vec<u8>, Vec<u8>)> {
        let rows = self.store.iter().filter(|(key, _)| key.starts_with(prefix));
        rows.map(|(key, value)| (key.clone(), value.clone()))
            .collect()
    }

    fn keep(&mut self, key: &[u8], value: Vec<u8>) {
        self.store.insert(key.to_vec(), value);
    }

    fn forget(&mut self, key: &[u8]) {
        self.store.remove(key);
    }

    fn refuse(&mut self, code: &str) {
        self.refused = Some(code.to_owned());
    }
}

/// Where the tests build: dry land in the middle of a sector.
fn place(generator: &Generator) -> SurfacePoint {
    let side = f64::from(generator.sphere().blocks().side());
    SurfacePoint::new(Sector::new(4).unwrap(), side * 0.41, side * 0.37)
}

/// A session standing on the ground at a column.
fn body(generator: &Generator, session: u32, point: SurfacePoint, level: Level) -> Who {
    Who {
        session,
        user: String::new(),
        name: String::new(),
        level,
        stance: Some(Stance {
            sector: point.sector.index() as u32,
            u: point.u as f32,
            v: point.v as f32,
            height_m: (seat::ground(generator, point) * BLOCK_M) as f32,
            ..Default::default()
        }),
    }
}

/// A world with a builder where the tests build, and a visitor far along
/// the sector from there.
fn stage() -> (Stage, SurfacePoint) {
    let generator = Generator::new(Recipe::new(1)).unwrap();
    let point = place(&generator);
    let far = SurfacePoint::new(point.sector, point.u + 4000.0, point.v);
    let here = vec![
        body(&generator, 1, point, Level::Builder),
        body(&generator, 2, far, Level::Anonymous),
    ];
    let stage = Stage {
        measure: Measure {
            sphere: generator.sphere(),
            moon_radius_m: worldgen::MOON_RADIUS_M,
        },
        ground: Some(generator),
        here,
        store: BTreeMap::new(),
        told: Vec::new(),
        refused: None,
    };
    (stage, point)
}

fn ask<M: Message>(cells: &mut Cells, stage: &mut Stage, session: u32, kind: &str, op: &M) {
    let who = stage
        .here
        .iter()
        .find(|who| who.session == session)
        .cloned()
        .unwrap();
    stage.refused = None;
    cells.op(kind, &op.encode_to_vec(), &who, stage);
}

fn seat_of(point: SurfacePoint) -> Option<wire::Seat> {
    Some(Seat::Sector(point.sector).wire())
}

fn open(cells: &mut Cells, stage: &mut Stage, session: u32, point: SurfacePoint) {
    let open = wire::Open {
        seat: seat_of(point),
        u: point.u,
        v: point.v,
    };
    ask(cells, stage, session, wire::OPEN, &open);
}

fn change(
    cells: &mut Cells,
    stage: &mut Stage,
    session: u32,
    point: SurfacePoint,
    made: &[Gesture],
) {
    let change = wire::Change {
        seat: seat_of(point),
        gestures: made
            .iter()
            .map(|&gesture| seat::gesture_wire(gesture))
            .collect(),
    };
    ask(cells, stage, session, wire::CHANGE, &change);
}

/// What a session that holds nothing is shown of the volumes near it.
fn seen(
    cells: &mut Cells,
    stage: &mut Stage,
    session: u32,
    held: Vec<wire::Held>,
) -> Vec<wire::Seen> {
    stage.told.clear();
    ask(cells, stage, session, wire::LOOK, &wire::Look { held });
    let seen = stage.told.iter().filter(|(kind, ..)| kind == wire::SEEN);
    seen.map(|(_, payload, to)| {
        assert_eq!(to, &[session]);
        wire::Seen::decode(payload.as_slice()).unwrap()
    })
    .collect()
}

/// The cells of a box as a session that looks is shown them.
fn shown(cells: &mut Cells, stage: &mut Stage, session: u32, span: Span) -> Vec<Cell> {
    let mut volumes = Volumes::new(PLOT_BITS);
    for volume in seen(cells, stage, session, Vec::new())
        .into_iter()
        .flat_map(|seen| seen.volumes)
    {
        let stood = volume.stood.unwrap();
        volumes.open([stood.plot_x, stood.plot_y], stood.low, stood.height);
        for chunk in volume.chunks {
            let cells = unpack(&chunk.cells, CHUNK_CELLS).unwrap();
            volumes.restore(Volumes::chunk_span([chunk.x, chunk.y, chunk.z]), &cells);
        }
    }
    volumes.cells(span)
}

/// A box of cells on the tests' plot, counted from its corner at the height
/// of the ground there.
fn boxed(stage: &Stage, point: SurfacePoint, from: [i32; 3], to: [i32; 3]) -> Span {
    let [x, y] = seat::plot_of(point).map(|n| n << PLOT_BITS);
    let z = seat::ground(stage.ground.as_ref().unwrap(), point).ceil() as i32 + 4;
    let at = |cell: [i32; 3]| [x + cell[0], y + cell[1], z + cell[2]];
    Span::between(at(from), at(to))
}

#[test]
fn a_volume_is_seated_by_the_ground_and_told_to_whoever_is_near() {
    let (mut stage, point) = stage();
    let mut cells = Cells::default();
    open(&mut cells, &mut stage, 1, point);
    assert_eq!(stage.refused, None);
    let [(kind, payload, to)] = stage.told.as_slice() else {
        panic!("one event: {:?}", stage.told.len());
    };
    assert_eq!((kind.as_str(), to.as_slice()), (wire::OPENED, &[1][..]));
    let opened = wire::Opened::decode(payload.as_slice()).unwrap();
    let stood = opened.stood.unwrap();
    let stand = seat::survey(stage.ground.as_ref().unwrap(), point).unwrap();
    assert_eq!(
        (stood.low, stood.height, stood.version),
        (stand.low, stand.height, 0)
    );
    assert_eq!([stood.plot_x, stood.plot_y], seat::plot_of(point));
    // Asked again, it stands as it stood, and the asker alone is told.
    stage.told.clear();
    open(&mut cells, &mut stage, 1, point);
    assert_eq!(stage.told.len(), 1);
    assert_eq!(stage.store.len(), 1);

    // The edge of a sector stays nature, and so does a world whose ground
    // the server does not hold.
    let edge = SurfacePoint::new(point.sector, 10.0, point.v);
    open(&mut cells, &mut stage, 1, edge);
    assert_eq!(stage.refused.as_deref(), Some("seam"));
    stage.ground = None;
    let beside = SurfacePoint::new(point.sector, point.u + 64.0, point.v);
    open(&mut cells, &mut stage, 1, beside);
    assert_eq!(stage.refused.as_deref(), Some("ground"));
}

#[test]
fn a_change_is_kept_told_to_who_is_near_and_shown_to_who_arrives() {
    let (mut stage, point) = stage();
    let mut cells = Cells::default();
    let span = boxed(&stage, point, [3, 3, 0], [20, 3, 2]);
    let made = [Gesture::Create { span, paint: 5 }];

    // Where no volume stands, a change lands nowhere.
    change(&mut cells, &mut stage, 1, point, &made);
    assert_eq!(stage.refused.as_deref(), Some("volume"));

    open(&mut cells, &mut stage, 1, point);
    stage.told.clear();
    change(&mut cells, &mut stage, 1, point, &made);
    assert_eq!(stage.refused, None);
    let [(kind, payload, to)] = stage.told.as_slice() else {
        panic!("one event");
    };
    // The builder is near and hears it; the visitor far away does not.
    assert_eq!((kind.as_str(), to.as_slice()), (wire::CHANGED, &[1][..]));
    let changed = wire::Changed::decode(payload.as_slice()).unwrap();
    assert_eq!((changed.session, changed.stood[0].version), (1, 1));
    assert_eq!(seat::gesture_from_wire(&changed.gestures[0]), Some(made[0]));

    // A new instance of the module reads it all back from the store, for
    // whoever looks from near: the visitor, walked over.
    let mut again = Cells::default();
    assert!(seen(&mut again, &mut stage, 2, Vec::new()).is_empty());
    stage.here[1].stance = stage.here[0].stance;
    let kept = shown(&mut again, &mut stage, 2, span);
    assert!(kept.iter().all(|cell| cell.paint() == Some(5)));
    // Held at the version it stands at, a look shows nothing more; held at
    // another, it is shown whole again; one that stands nowhere is gone.
    let [x, y] = seat::plot_of(point);
    let held = |plot_x: i32, version: u64| wire::Held {
        seat: seat_of(point),
        plot_x,
        plot_y: y,
        version,
    };
    assert!(seen(&mut again, &mut stage, 2, vec![held(x, 1)]).is_empty());
    assert_eq!(seen(&mut again, &mut stage, 2, vec![held(x, 0)]).len(), 1);
    let looked = seen(&mut again, &mut stage, 2, vec![held(x, 1), held(x + 9, 3)]);
    let [only] = looked.as_slice() else {
        panic!("one answer");
    };
    assert!(only.volumes.is_empty());
    assert_eq!(only.gone, vec![held(x + 9, 3)]);
}

#[test]
fn a_change_is_bounded_and_what_is_no_message_is_refused() {
    let (mut stage, point) = stage();
    let mut cells = Cells::default();
    open(&mut cells, &mut stage, 1, point);
    let wide = boxed(&stage, point, [0, 0, 0], [300, 0, 0]);
    change(
        &mut cells,
        &mut stage,
        1,
        point,
        &[Gesture::Delete { span: wide }],
    );
    assert_eq!(stage.refused.as_deref(), Some("reach"));
    let who = stage.here[0].clone();
    for kind in [wire::OPEN, wire::CHANGE, wire::LOOK] {
        stage.refused = None;
        cells.op(kind, &[0xff, 0xff, 0xff], &who, &mut stage);
        assert_eq!(stage.refused.as_deref(), Some("message"), "{kind}");
    }
    // A change that changes nothing lands, and nobody is told.
    stage.told.clear();
    let air = boxed(&stage, point, [1, 1, 1], [2, 2, 2]);
    change(
        &mut cells,
        &mut stage,
        1,
        point,
        &[Gesture::Delete { span: air }],
    );
    assert_eq!((stage.refused.clone(), stage.told.len()), (None, 0));
}

#[test]
fn taking_back_leaves_what_someone_else_changed_since() {
    let (mut stage, point) = stage();
    stage.here[1] = Who {
        session: 2,
        level: Level::Builder,
        ..stage.here[0].clone()
    };
    let mut cells = Cells::default();
    open(&mut cells, &mut stage, 1, point);
    let row = boxed(&stage, point, [3, 3, 0], [8, 3, 0]);
    let one = boxed(&stage, point, [5, 3, 0], [5, 3, 0]);
    change(
        &mut cells,
        &mut stage,
        1,
        point,
        &[Gesture::Create {
            span: row,
            paint: 1,
        }],
    );
    // Someone else repaints one cell of the row.
    change(
        &mut cells,
        &mut stage,
        2,
        point,
        &[Gesture::Paint {
            span: one,
            paint: 9,
        }],
    );

    stage.told.clear();
    ask(
        &mut cells,
        &mut stage,
        1,
        wire::TAKE_BACK,
        &wire::TakeBack {},
    );
    assert_eq!(stage.refused, None);
    let (kind, payload, to) = stage.told.last().unwrap();
    assert_eq!(
        (kind.as_str(), to.as_slice()),
        (wire::RESTORED, &[1, 2][..])
    );
    let restored = wire::Restored::decode(payload.as_slice()).unwrap();
    assert_eq!(restored.stood[0].version, 3);
    let now = shown(&mut cells, &mut stage, 2, row);
    let paints: Vec<Option<u8>> = now.iter().map(|cell| cell.paint()).collect();
    assert_eq!(paints, [None, None, Some(9), None, None, None]);
    assert_eq!(unpack(&restored.cells, 6), Some(now));

    // Put back, the row is whole again around the cell that was repainted.
    ask(&mut cells, &mut stage, 1, wire::PUT_BACK, &wire::PutBack {});
    let paints: Vec<Option<u8>> = shown(&mut cells, &mut stage, 2, row)
        .iter()
        .map(|cell| cell.paint())
        .collect();
    assert_eq!(
        paints,
        [Some(1), Some(1), Some(9), Some(1), Some(1), Some(1)]
    );
    // There is nothing more to put back, and one who left takes nothing back.
    ask(&mut cells, &mut stage, 1, wire::PUT_BACK, &wire::PutBack {});
    assert_eq!(stage.refused.as_deref(), Some("none"));
    cells.gone(1);
    ask(
        &mut cells,
        &mut stage,
        1,
        wire::TAKE_BACK,
        &wire::TakeBack {},
    );
    assert_eq!(stage.refused.as_deref(), Some("none"));
}

#[test]
fn the_cells_are_a_builders_to_change_and_anyones_to_look_at() {
    let ops = Cells::default().ops();
    let level = |kind: &str| ops.iter().find(|op| op.kind == kind).map(|op| op.level);
    for kind in [wire::OPEN, wire::CHANGE, wire::TAKE_BACK, wire::PUT_BACK] {
        assert_eq!(level(kind), Some(Level::Builder), "{kind}");
    }
    assert_eq!(level(wire::LOOK), Some(Level::Anonymous));
}
