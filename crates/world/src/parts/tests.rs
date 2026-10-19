//! The parts' system over a room a test holds: a store in memory, and
//! whoever a test puts in it.

use std::collections::BTreeMap;

use glam::DQuat;
use topology::{BLOCK_M, Sector, SurfacePoint};
use worldgen::{Generator, Recipe};

use super::*;
use crate::{Measure, Moment};

/// A room that keeps what it is told and what it is given to keep, as the
/// server does once an op has been applied.
struct Stage {
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
        None
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

fn body(session: u32, point: SurfacePoint, level: Level) -> Who {
    Who {
        session,
        user: String::new(),
        name: String::new(),
        level,
        stance: Some(Stance {
            sector: point.sector.index() as u32,
            u: point.u as f32,
            v: point.v as f32,
            height_m: 0.0,
            ..Default::default()
        }),
    }
}

/// A world with a builder in the middle of a sector and a visitor far
/// along it.
fn stage() -> (Stage, SurfacePoint) {
    let generator = Generator::new(Recipe::new(1)).unwrap();
    let side = f64::from(generator.sphere().blocks().side());
    let point = SurfacePoint::new(Sector::new(4).unwrap(), side * 0.41, side * 0.37);
    let far = SurfacePoint::new(point.sector, point.u + 8000.0, point.v);
    let here = vec![
        body(1, point, Level::Builder),
        body(2, far, Level::Anonymous),
    ];
    let stage = Stage {
        measure: Measure {
            sphere: generator.sphere(),
            moon: generator.moon(),
        },
        here,
        store: BTreeMap::new(),
        told: Vec::new(),
        refused: None,
    };
    (stage, point)
}

fn cube(point: SurfacePoint, h: f64) -> wire::Part {
    let part = part::Part {
        shape: part::Shape::Cube,
        pose: part::Pose {
            at: [point.u, point.v, h],
            rotation: DQuat::from_rotation_z(0.5),
            scale: [2.0, 2.0, 2.0],
        },
        paint: 11,
        solid: true,
        shine: 0.0,
        reach_m: 0.0,
    };
    seat::part_wire(Seat::Sector(point.sector), &part)
}

/// Takes back or puts back a session's last change through the owner's op.
/// Whether it landed.
fn restore(parts: &mut Parts, stage: &mut Stage, who: &Who, back: bool) -> bool {
    stage.refused = None;
    match back {
        true => parts.op(
            wire::TAKE_BACK,
            &wire::TakeBack {}.encode_to_vec(),
            who,
            stage,
        ),
        false => parts.op(
            wire::PUT_BACK,
            &wire::PutBack {}.encode_to_vec(),
            who,
            stage,
        ),
    }
    stage.refused.is_none()
}

fn told<M: Message + Default>(stage: &mut Stage, kind: &str) -> Vec<(M, Vec<u32>)> {
    let told = core::mem::take(&mut stage.told);
    told.into_iter()
        .filter(|(said, _, _)| said == kind)
        .map(|(_, payload, to)| (M::decode(payload.as_slice()).unwrap(), to))
        .collect()
}

#[test]
fn a_part_placed_is_named_told_to_who_is_near_and_kept() {
    let (mut stage, point) = stage();
    let mut parts = Parts::default();
    let who = stage.here[0].clone();
    let place = wire::Place {
        part: Some(cube(point, 4.0)),
    };
    parts.op(wire::PLACE, &place.encode_to_vec(), &who, &mut stage);
    assert_eq!(stage.refused, None);
    let placed = told::<wire::Placed>(&mut stage, wire::PLACED);
    assert_eq!(placed.len(), 1);
    let (placed, to) = &placed[0];
    assert_eq!(
        to,
        &[1],
        "the builder near it hears it, the visitor far off does not"
    );
    let record = placed.record.as_ref().unwrap();
    assert_eq!((placed.session, record.id, record.version), (1, 1, 0));
    assert_eq!(record.part.as_ref().unwrap().paint, 11);
    // Kept under its name, and the next name after it.
    assert!(stage.store.contains_key(&record_key(1)));
    assert_eq!(
        stage.store.get(NEXT_KEY).unwrap(),
        &2u64.to_be_bytes().to_vec()
    );

    // A new instance reads what stands and gives the next name, never one
    // given before, even to a part since removed.
    let mut again = Parts::default();
    let remove = wire::Remove { id: 1 };
    again.op(wire::REMOVE, &remove.encode_to_vec(), &who, &mut stage);
    assert_eq!(told::<wire::Removed>(&mut stage, wire::REMOVED)[0].0.id, 1);
    assert!(stage.scan(b"p").is_empty());
    again.op(wire::PLACE, &place.encode_to_vec(), &who, &mut stage);
    let placed = told::<wire::Placed>(&mut stage, wire::PLACED);
    assert_eq!(placed[0].0.record.as_ref().unwrap().id, 2);
}

#[test]
fn a_part_set_counts_a_version_and_a_pose_outside_its_sector_is_refused() {
    let (mut stage, point) = stage();
    let mut parts = Parts::default();
    let who = stage.here[0].clone();
    let place = wire::Place {
        part: Some(cube(point, 4.0)),
    };
    parts.op(wire::PLACE, &place.encode_to_vec(), &who, &mut stage);
    stage.told.clear();
    let mut moved = cube(point, 9.0);
    moved.paint = 3;
    let set = wire::Set {
        id: 1,
        part: Some(moved),
    };
    parts.op(wire::SET, &set.encode_to_vec(), &who, &mut stage);
    assert_eq!(stage.refused, None);
    let changed = told::<wire::Changed>(&mut stage, wire::CHANGED);
    let record = changed[0].0.record.as_ref().unwrap();
    assert_eq!(
        (record.version, record.part.as_ref().unwrap().paint),
        (1, 3)
    );
    let kept = wire::Record::decode(stage.store.get(&record_key(1)).unwrap().as_slice()).unwrap();
    assert_eq!(kept.version, 1);

    let outside = SurfacePoint::new(point.sector, -3.0, point.v);
    let set = wire::Set {
        id: 1,
        part: Some(cube(outside, 4.0)),
    };
    parts.op(wire::SET, &set.encode_to_vec(), &who, &mut stage);
    assert_eq!(stage.refused.take(), Some("pose".into()));
    let set = wire::Set {
        id: 9,
        part: Some(cube(point, 4.0)),
    };
    parts.op(wire::SET, &set.encode_to_vec(), &who, &mut stage);
    assert_eq!(stage.refused.take(), Some("gone".into()));
    let mut flat = cube(point, 4.0);
    flat.pose.as_mut().unwrap().sx = 0.0;
    let place = wire::Place { part: Some(flat) };
    parts.op(wire::PLACE, &place.encode_to_vec(), &who, &mut stage);
    assert_eq!(stage.refused.take(), Some("message".into()));
}

#[test]
fn a_change_is_taken_back_through_the_owner_and_put_back() {
    let (mut stage, point) = stage();
    let mut parts = Parts::default();
    let who = stage.here[0].clone();
    let place = wire::Place {
        part: Some(cube(point, 4.0)),
    };
    parts.op(wire::PLACE, &place.encode_to_vec(), &who, &mut stage);
    let mut moved = cube(point, 9.0);
    moved.paint = 3;
    let set = wire::Set {
        id: 1,
        part: Some(moved),
    };
    parts.op(wire::SET, &set.encode_to_vec(), &who, &mut stage);
    stage.told.clear();
    // The set goes back: the part is as placed, one version on, and told.
    assert!(restore(&mut parts, &mut stage, &who, true));
    let changed = told::<wire::Changed>(&mut stage, wire::CHANGED);
    let record = changed[0].0.record.as_ref().unwrap();
    assert_eq!(
        (record.version, record.part.as_ref().unwrap().paint),
        (2, 11)
    );
    // The place goes back: the part is gone and told so.
    assert!(restore(&mut parts, &mut stage, &who, true));
    assert_eq!(told::<wire::Removed>(&mut stage, wire::REMOVED)[0].0.id, 1);
    assert!(stage.scan(b"p").is_empty());
    // Nothing more to take back.
    assert!(!restore(&mut parts, &mut stage, &who, true));
    // Put back: it stands again under its name, then the set is put back.
    assert!(restore(&mut parts, &mut stage, &who, false));
    let placed = told::<wire::Placed>(&mut stage, wire::PLACED);
    assert_eq!(placed[0].0.record.as_ref().unwrap().id, 1);
    assert!(restore(&mut parts, &mut stage, &who, false));
    let changed = told::<wire::Changed>(&mut stage, wire::CHANGED);
    assert_eq!(
        changed[0]
            .0
            .record
            .as_ref()
            .unwrap()
            .part
            .as_ref()
            .unwrap()
            .paint,
        3
    );
    assert!(!restore(&mut parts, &mut stage, &who, false));
    // Removed by someone else since: the change is gone and nothing is done.
    let other = stage.here[1].clone();
    let remove = wire::Remove { id: 1 };
    parts.op(wire::REMOVE, &remove.encode_to_vec(), &other, &mut stage);
    stage.told.clear();
    assert!(!restore(&mut parts, &mut stage, &who, true));
    assert!(stage.told.is_empty());
}

#[test]
fn a_look_is_answered_with_what_is_lacking_near_and_what_is_gone() {
    let (mut stage, point) = stage();
    let mut parts = Parts::default();
    let who = stage.here[0].clone();
    for h in [4.0, 6.0] {
        let place = wire::Place {
            part: Some(cube(point, h)),
        };
        parts.op(wire::PLACE, &place.encode_to_vec(), &who, &mut stage);
    }
    // Along the sector, past holding distance.
    let far = SurfacePoint::new(point.sector, point.u + (HOLD_M + 512.0) / BLOCK_M, point.v);
    let place = wire::Place {
        part: Some(cube(far, 4.0)),
    };
    parts.op(wire::PLACE, &place.encode_to_vec(), &who, &mut stage);
    stage.told.clear();
    // Holding the first as it stands, the second stale, and one that is gone.
    let look = wire::Look {
        held: vec![
            wire::Held { id: 1, version: 0 },
            wire::Held { id: 2, version: 7 },
            wire::Held { id: 40, version: 0 },
        ],
    };
    parts.op(wire::LOOK, &look.encode_to_vec(), &who, &mut stage);
    let seen = told::<wire::Seen>(&mut stage, wire::SEEN);
    let (seen, to) = &seen[0];
    assert_eq!(to, &[1]);
    let ids: Vec<u64> = seen.records.iter().map(|record| record.id).collect();
    assert_eq!(
        ids,
        vec![2],
        "the stale one near; not the held one, not the far one"
    );
    assert_eq!(seen.gone, vec![40]);
    // The visitor far off lacks everything near it: the far part alone.
    let visitor = body(2, far, Level::Anonymous);
    let look = wire::Look { held: vec![] };
    parts.op(wire::LOOK, &look.encode_to_vec(), &visitor, &mut stage);
    let seen = told::<wire::Seen>(&mut stage, wire::SEEN);
    let ids: Vec<u64> = seen[0].0.records.iter().map(|record| record.id).collect();
    assert_eq!(ids, vec![3]);
}
