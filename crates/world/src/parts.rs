//! The parts of a world where the world is decided (DECISIONS 111): a
//! system of the core, an owner as a plugin is. It names a part a session
//! places, keeps it in its store, sets and removes it, tells whoever is
//! near, and shows a client that looks the parts near it that it lacks.
//!
//! A part is one row: its record under its name. What is held in memory is
//! read again by a new instance: every record, and the next name to give.

use std::collections::{BTreeMap, HashMap};

use prost::Message;
use protocol::parts as wire;
use protocol::{Body, Stance};
use seat::{Seat, TELL_M};

use crate::{Level, Measure, Op, Plugin, Room, Who};

/// The version of the parts' wire.
pub const VERSION: u32 = 1;

/// How near a body a part is for its client to hold it, metres: the step
/// afar the cells are held to, since a part is a record and not chunks.
pub const HOLD_M: f64 = 2048.0;
/// The most records one look is answered with.
const LOOK_RECORDS: usize = 4096;

/// The key a part's record is kept under: a tag and its name.
fn record_key(id: u64) -> Vec<u8> {
    let mut key = vec![b'p'];
    key.extend(id.to_be_bytes());
    key
}

/// The key the next name to give is kept under.
const NEXT_KEY: &[u8] = b"next";

/// A part as the world keeps it.
#[derive(Clone, PartialEq, Debug)]
struct Kept {
    seat: Seat,
    part: part::Part,
    version: u64,
}

impl Kept {
    fn record(&self, id: u64) -> wire::Record {
        wire::Record {
            id,
            part: Some(seat::part_wire(self.seat, &self.part)),
            version: self.version,
        }
    }

    fn from_record(record: &wire::Record) -> Option<Kept> {
        let (seat, part) = seat::part_from_wire(record.part.as_ref()?)?;
        Some(Kept {
            seat,
            part,
            version: record.version,
        })
    }
}

/// A change that landed, as what takes it back and puts it back.
enum Change {
    /// A part placed, by name.
    Placed(u64),
    /// A part set: as it was before.
    Set(u64, Kept),
    /// A part removed: as it was.
    Removed(u64, Kept),
}

/// Changes a session may take back, and the most of them kept.
const HISTORY: usize = 100;

/// What a session may take back, the last change last, and put back.
#[derive(Default)]
struct History {
    done: Vec<Change>,
    undone: Vec<Change>,
}

impl History {
    /// Keeps a change that landed as the last to take back. Sets of one part
    /// in a row are one change, back to how it stood before the first: a
    /// drag sets a part many times a second.
    fn did(&mut self, change: Change) {
        self.undone.clear();
        if let (Change::Set(id, _), Some(Change::Set(last, _))) = (&change, self.done.last())
            && id == last
        {
            return;
        }
        self.done.push(change);
        if self.done.len() > HISTORY {
            self.done.remove(0);
        }
    }
}

/// The system. The host calls it one op at a time.
#[derive(Default)]
pub struct Parts {
    /// Every part of the world, by name: read from the store by the first
    /// op, and kept in step with it.
    index: Option<BTreeMap<u64, Kept>>,
    /// The next name to give, read with the index. Never one given before.
    next: u64,
    histories: HashMap<u32, History>,
}

/// The system as the server's module holds it.
pub fn system() -> Box<dyn Plugin> {
    Box::new(Parts::default())
}

impl Parts {
    fn index(&mut self, room: &mut dyn Room) -> &mut BTreeMap<u64, Kept> {
        if self.index.is_none() {
            let rows = room.scan(b"p");
            let kept = rows.iter().filter_map(|(_, value)| {
                let record = wire::Record::decode(value.as_slice()).ok()?;
                Some((record.id, Kept::from_record(&record)?))
            });
            let index: BTreeMap<u64, Kept> = kept.collect();
            let last = index.keys().next_back().copied().unwrap_or(0);
            let next = room
                .get(NEXT_KEY)
                .and_then(|bytes| Some(u64::from_be_bytes(bytes.try_into().ok()?)))
                .unwrap_or(0);
            self.next = next.max(last + 1).max(1);
            self.index = Some(index);
        }
        self.index.as_mut().expect("read just now")
    }

    /// Whether a part may stand on its seat as its pose says.
    fn inside(measure: &Measure, seat: Seat, part: &part::Part) -> bool {
        seat::inside(measure.body(seat.body()), &part.pose)
    }

    fn keep(&mut self, room: &mut dyn Room, id: u64, kept: &Kept) {
        room.keep(&record_key(id), kept.record(id).encode_to_vec());
        self.index(room).insert(id, kept.clone());
    }

    fn place(&mut self, payload: &[u8], who: &Who, room: &mut dyn Room) {
        let Ok(place) = wire::Place::decode(payload) else {
            return room.refuse("message");
        };
        let Some((seat, part)) = place.part.as_ref().and_then(seat::part_from_wire) else {
            return room.refuse("message");
        };
        if !Self::inside(&room.measure(), seat, &part) {
            return room.refuse("pose");
        }
        self.index(room);
        let id = self.next;
        self.next += 1;
        room.keep(NEXT_KEY, self.next.to_be_bytes().to_vec());
        let kept = Kept {
            seat,
            part,
            version: 0,
        };
        self.keep(room, id, &kept);
        self.tell_placed(room, who, id, &kept);
        self.histories
            .entry(who.session)
            .or_default()
            .did(Change::Placed(id));
    }

    /// Says a part placed to whoever is near, and to the asker whether
    /// near or not: it is waiting for the name.
    fn tell_placed(&self, room: &mut dyn Room, who: &Who, id: u64, kept: &Kept) {
        let mut to = near(room, kept.seat, &kept.part, TELL_M);
        if !to.contains(&who.session) {
            to.push(who.session);
        }
        let placed = wire::Placed {
            session: who.session,
            record: Some(kept.record(id)),
        };
        room.tell(wire::PLACED, placed.encode_to_vec(), &to);
    }

    /// Sets a part as `kept` says, one version on, and tells whoever is near
    /// where it stood and where it stands.
    fn apply_set(&mut self, room: &mut dyn Room, who: &Who, id: u64, before: &Kept, kept: Kept) {
        self.keep(room, id, &kept);
        let mut to = near(room, before.seat, &before.part, TELL_M);
        for session in near(room, kept.seat, &kept.part, TELL_M) {
            if !to.contains(&session) {
                to.push(session);
            }
        }
        let changed = wire::Changed {
            session: who.session,
            record: Some(kept.record(id)),
        };
        room.tell(wire::CHANGED, changed.encode_to_vec(), &to);
    }

    /// Takes a part out of the world and tells whoever is near. What it was.
    fn take_out(&mut self, room: &mut dyn Room, who: &Who, id: u64) -> Option<Kept> {
        let kept = self.index(room).remove(&id)?;
        room.forget(&record_key(id));
        let removed = wire::Removed {
            session: who.session,
            id,
        };
        let to = near(room, kept.seat, &kept.part, TELL_M);
        room.tell(wire::REMOVED, removed.encode_to_vec(), &to);
        Some(kept)
    }

    fn set(&mut self, payload: &[u8], who: &Who, room: &mut dyn Room) {
        let Ok(set) = wire::Set::decode(payload) else {
            return room.refuse("message");
        };
        let Some((seat, part)) = set.part.as_ref().and_then(seat::part_from_wire) else {
            return room.refuse("message");
        };
        if !Self::inside(&room.measure(), seat, &part) {
            return room.refuse("pose");
        }
        let Some(before) = self.index(room).get(&set.id).cloned() else {
            return room.refuse("gone");
        };
        let kept = Kept {
            seat,
            part,
            version: before.version + 1,
        };
        self.apply_set(room, who, set.id, &before, kept);
        self.histories
            .entry(who.session)
            .or_default()
            .did(Change::Set(set.id, before));
    }

    fn remove(&mut self, payload: &[u8], who: &Who, room: &mut dyn Room) {
        let Ok(remove) = wire::Remove::decode(payload) else {
            return room.refuse("message");
        };
        let Some(kept) = self.take_out(room, who, remove.id) else {
            return room.refuse("gone");
        };
        self.histories
            .entry(who.session)
            .or_default()
            .did(Change::Removed(remove.id, kept));
    }

    /// Takes back the last change of a session, or puts back the last it
    /// took back. A part placed is taken out and stands again under the
    /// same name; a part set goes back to how it was, one version on; a
    /// part removed stands again as it was. A part someone else removed
    /// since, or set since, is left as they left it: the change is then
    /// gone from the history and nothing is done. False with nothing to
    /// take back.
    fn restore(&mut self, back: bool, who: &Who, room: &mut dyn Room) -> bool {
        let history = self.histories.entry(who.session).or_default();
        let change = match back {
            true => history.done.pop(),
            false => history.undone.pop(),
        };
        let Some(change) = change else {
            return false;
        };
        let inverse = match change {
            Change::Placed(id) => self
                .take_out(room, who, id)
                .map(|kept| Change::Removed(id, kept)),
            Change::Removed(id, kept) => {
                if self.index(room).contains_key(&id) {
                    None
                } else {
                    let kept = Kept {
                        version: kept.version + 1,
                        ..kept
                    };
                    self.keep(room, id, &kept);
                    self.tell_placed(room, who, id, &kept);
                    Some(Change::Placed(id))
                }
            }
            Change::Set(id, before) => match self.index(room).get(&id).cloned() {
                Some(now) => {
                    let kept = Kept {
                        version: now.version + 1,
                        ..before
                    };
                    self.apply_set(room, who, id, &now, kept);
                    Some(Change::Set(id, now))
                }
                None => None,
            },
        };
        let history = self.histories.entry(who.session).or_default();
        match (inverse, back) {
            (Some(inverse), true) => history.undone.push(inverse),
            (Some(inverse), false) => history.done.push(inverse),
            (None, _) => return false,
        }
        true
    }

    /// Answers what a client holds with what it lacks: every part near its
    /// body that it has not, or has at another version, the nearest first,
    /// and those it holds that stand no more.
    fn look(&mut self, payload: &[u8], who: &Who, room: &mut dyn Room) {
        let Ok(look) = wire::Look::decode(payload) else {
            return room.refuse("message");
        };
        let measure = room.measure();
        let held: BTreeMap<u64, u64> = look.held.iter().map(|at| (at.id, at.version)).collect();
        let index = self.index(room);
        let gone: Vec<u64> = held
            .keys()
            .copied()
            .filter(|id| !index.contains_key(id))
            .collect();
        let from = who
            .stance
            .as_ref()
            .and_then(|stance| place(&measure, stance));
        let mut lacking: Vec<(f64, u64, wire::Record)> = index
            .iter()
            .filter_map(|(&id, kept)| {
                let away_m = away_m(&measure, kept.seat, &kept.part, from?)?;
                let holds = held.get(&id) == Some(&kept.version);
                (away_m <= HOLD_M && !holds).then(|| (away_m, id, kept.record(id)))
            })
            .collect();
        lacking.sort_by(|a, b| a.0.total_cmp(&b.0));
        lacking.truncate(LOOK_RECORDS);
        if gone.is_empty() && lacking.is_empty() {
            return;
        }
        let seen = wire::Seen {
            records: lacking.into_iter().map(|(_, _, record)| record).collect(),
            gone,
        };
        room.tell(wire::SEEN, seen.encode_to_vec(), &[who.session]);
    }
}

/// Where a stance is: the body it is on, and metres from that body's centre.
fn place(measure: &Measure, stance: &Stance) -> Option<(Body, [f64; 3])> {
    Some((stance.body(), measure.position(stance)?))
}

/// How far a place is from a part's anchor, metres. `None` from another
/// body: nothing there is near.
fn away_m(
    measure: &Measure,
    seat: Seat,
    part: &part::Part,
    (body, at): (Body, [f64; 3]),
) -> Option<f64> {
    if body != seat.body() {
        return None;
    }
    let anchor = seat::seated(measure.body(seat.body()), seat.sector(), &part.pose).translation;
    let [dx, dy, dz] = [anchor.x - at[0], anchor.y - at[1], anchor.z - at[2]];
    Some(libm::sqrt(dx * dx + dy * dy + dz * dz))
}

/// The sessions whose bodies are within reach of a part.
fn near(room: &dyn Room, seat: Seat, part: &part::Part, reach_m: f64) -> Vec<u32> {
    let measure = room.measure();
    room.sessions()
        .iter()
        .filter(|who| {
            let from = who
                .stance
                .as_ref()
                .and_then(|stance| place(&measure, stance));
            from.is_some_and(|from| {
                away_m(&measure, seat, part, from).is_some_and(|away_m| away_m <= reach_m)
            })
        })
        .map(|who| who.session)
        .collect()
}

impl Plugin for Parts {
    fn name(&self) -> &'static str {
        wire::OWNER
    }

    fn version(&self) -> u32 {
        VERSION
    }

    /// Placing, setting and removing a part is a builder's and an admin's,
    /// and taking that back. Anyone looks.
    fn ops(&self) -> Vec<Op> {
        let builder = [
            wire::PLACE,
            wire::SET,
            wire::REMOVE,
            wire::TAKE_BACK,
            wire::PUT_BACK,
        ];
        let mut ops: Vec<Op> = builder
            .into_iter()
            .map(|kind| Op {
                kind,
                level: Level::Builder,
            })
            .collect();
        ops.push(Op {
            kind: wire::LOOK,
            level: Level::Anonymous,
        });
        ops
    }

    fn op(&mut self, kind: &str, payload: &[u8], who: &Who, room: &mut dyn Room) {
        match kind {
            wire::PLACE => self.place(payload, who, room),
            wire::SET => self.set(payload, who, room),
            wire::REMOVE => self.remove(payload, who, room),
            wire::TAKE_BACK | wire::PUT_BACK => {
                if !self.restore(kind == wire::TAKE_BACK, who, room) {
                    room.refuse("none");
                }
            }
            wire::LOOK => self.look(payload, who, room),
            _ => {}
        }
    }

    fn gone(&mut self, session: u32) {
        self.histories.remove(&session);
    }
}

#[cfg(test)]
mod tests;
