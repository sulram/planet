//! The cells of a world where the world is decided (DECISIONS 106, 107,
//! 110): a system of the core, an owner as a plugin is. It seats a volume by
//! the ground under its plot, applies the gestures a session asks for, keeps
//! what they made in its store, tells whoever is near, closes a volume, and
//! takes a change back for the one who made it.
//!
//! It holds no cell between two ops: a change reads the chunks it reaches
//! from the store and writes back those it changed. What it holds in memory
//! is what a new instance does without or reads again: where every volume
//! stands, and what each session may take back.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use prost::Message;
use protocol::cells as wire;
use seat::{HOLD_M, PLOT_BITS, Seat, Stand, TELL_M};
use topology::{Sector, SurfacePoint};
use voxel::{CHUNK, Cell, Gesture, Span, Volumes, pack, unpack};

use crate::{Level, Op, Plugin, Room, Who};

/// The version of the cells' wire.
pub const VERSION: u32 = 1;

/// The most gestures one change carries: a solid platform of a plot on the
/// steepest ground is fewer.
const GESTURES: usize = 8192;
/// The most cells a gesture reaches along the ground, and up: a few plots,
/// and as high as a volume rises over its ground.
const REACH: [u32; 3] = [256, 256, 1024];
/// Changes a session may take back, and the most bytes they are kept in.
const HISTORY: usize = 100;
const HISTORY_BYTES: usize = 4 << 20;
/// The most bytes of volumes one look is answered with: what is left is
/// asked for again by the next.
const LOOK_BYTES: usize = 4 << 20;
/// The most bytes of chunks one message of a volume carries.
const SEEN_BYTES: usize = 400_000;
const CHUNK_CELLS: usize = (CHUNK * CHUNK * CHUNK) as usize;

type Plot = (Seat, [i32; 2]);

/// A volume as the world keeps it: where it stands, and how many changes
/// it has taken.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Stood {
    stand: Stand,
    version: u64,
}

impl Stood {
    fn wire(self, plot: [i32; 2]) -> wire::Stood {
        wire::Stood {
            plot_x: plot[0],
            plot_y: plot[1],
            low: self.stand.low,
            height: self.stand.height,
            version: self.version,
        }
    }
}

/// A change that landed, as what takes it back.
enum Kept {
    /// Gestures, and the cells of their box before them, packed.
    Change {
        seat: Seat,
        span: Span,
        gestures: Vec<Gesture>,
        before: Vec<u8>,
    },
    /// A volume closed: how it stood, and every chunk the store held of it,
    /// under its key.
    Closed {
        seat: Seat,
        plot: [i32; 2],
        stood: Stood,
        chunks: Vec<(Vec<u8>, Vec<u8>)>,
    },
}

impl Kept {
    fn bytes(&self) -> usize {
        match self {
            Kept::Change {
                gestures, before, ..
            } => before.len() + gestures.len() * size_of::<Gesture>(),
            Kept::Closed { chunks, .. } => chunks
                .iter()
                .map(|(key, cells)| key.len() + cells.len())
                .sum(),
        }
    }
}

/// What a session may take back, the last change last, and put back.
#[derive(Default)]
struct History {
    done: Vec<Kept>,
    undone: Vec<Kept>,
}

impl History {
    /// Keeps a change that landed as the last to take back, and lets go of
    /// the oldest over what a session may keep.
    fn did(&mut self, kept: Kept) {
        self.done.push(kept);
        let mut bytes: usize = self.done.iter().map(Kept::bytes).sum();
        while self.done.len() > HISTORY || (bytes > HISTORY_BYTES && self.done.len() > 1) {
            bytes -= self.done.remove(0).bytes();
        }
    }
}

/// The system. The host calls it one op at a time.
#[derive(Default)]
pub struct Cells {
    /// Where every volume of the world stands: read from the store by the
    /// first op, and kept in step with it.
    index: Option<BTreeMap<Plot, Stood>>,
    histories: HashMap<u32, History>,
}

/// The system as the server's module holds it.
pub fn system() -> Box<dyn Plugin> {
    Box::new(Cells::default())
}

fn plot_key(tag: u8, sector: Sector, plot: [i32; 2]) -> Vec<u8> {
    let mut key = vec![tag, sector.index() as u8];
    key.extend(plot[0].to_be_bytes());
    key.extend(plot[1].to_be_bytes());
    key
}

/// The key a volume's record is kept under.
fn volume_key(sector: Sector, plot: [i32; 2]) -> Vec<u8> {
    plot_key(b'v', sector, plot)
}

/// The start of the keys a volume's chunks are kept under.
fn chunks_key(sector: Sector, plot: [i32; 2]) -> Vec<u8> {
    plot_key(b'c', sector, plot)
}

/// The key a chunk is kept under: its volume's, and its lowest corner.
fn chunk_key(sector: Sector, chunk: [i32; 3]) -> Vec<u8> {
    let plot = [chunk[0] >> PLOT_BITS, chunk[1] >> PLOT_BITS];
    let mut key = chunks_key(sector, plot);
    for at in chunk {
        key.extend(at.to_be_bytes());
    }
    key
}

fn number(bytes: &[u8]) -> Option<i32> {
    Some(i32::from_be_bytes(bytes.try_into().ok()?))
}

impl Cells {
    /// Where every volume stands, read from the store the first time.
    fn index(&mut self, room: &mut dyn Room) -> &mut BTreeMap<Plot, Stood> {
        self.index.get_or_insert_with(|| {
            let rows = room.scan(b"v");
            let volumes = rows.iter().filter_map(|(key, value)| {
                let sector = Sector::new(*key.get(1)?)?;
                let plot = [number(key.get(2..6)?)?, number(key.get(6..10)?)?];
                let kept = wire::Stood::decode(value.as_slice()).ok()?;
                let stand = Stand {
                    low: kept.low,
                    height: kept.height,
                };
                let stood = Stood {
                    stand,
                    version: kept.version,
                };
                Some(((Seat::Sector(sector), plot), stood))
            });
            volumes.collect()
        })
    }

    /// The volumes that boxes of a sector's cells reach, each opened where
    /// it stands, with the chunks the boxes reach read from the store.
    fn load(&mut self, room: &mut dyn Room, sector: Sector, spans: &[Span]) -> Volumes {
        let index = self.index(room);
        let mut volumes = Volumes::new(PLOT_BITS);
        for span in spans {
            let lo = [span.min[0] >> PLOT_BITS, span.min[1] >> PLOT_BITS];
            let hi = [span.max[0] >> PLOT_BITS, span.max[1] >> PLOT_BITS];
            for plot in (lo[1]..=hi[1]).flat_map(|y| (lo[0]..=hi[0]).map(move |x| [x, y])) {
                if let Some(stood) = index.get(&(Seat::Sector(sector), plot)) {
                    volumes.open(plot, stood.stand.low, stood.stand.height);
                }
            }
        }
        let chunks: BTreeSet<[i32; 3]> = spans
            .iter()
            .flat_map(|&span| volumes.chunks_in(span))
            .collect();
        for chunk in chunks {
            let kept = room.get(&chunk_key(sector, chunk));
            if let Some(cells) = kept.and_then(|packed| unpack(&packed, CHUNK_CELLS)) {
                volumes.restore(Volumes::chunk_span(chunk), &cells);
            }
        }
        volumes
    }

    /// Keeps the chunks a change reached as they are now, counts the change
    /// in every volume it reached, and says how each stands.
    fn save(
        &mut self,
        room: &mut dyn Room,
        sector: Sector,
        volumes: &Volumes,
        changed: Span,
    ) -> Vec<([i32; 2], Stood)> {
        for chunk in volumes.chunks_in(changed) {
            let cells = volumes.cells(Volumes::chunk_span(chunk));
            let key = chunk_key(sector, chunk);
            match cells.iter().all(|cell| cell.is_air()) {
                true => room.forget(&key),
                false => room.keep(&key, pack(&cells)),
            }
        }
        let lo = [changed.min[0] >> PLOT_BITS, changed.min[1] >> PLOT_BITS];
        let hi = [changed.max[0] >> PLOT_BITS, changed.max[1] >> PLOT_BITS];
        let index = self.index(room);
        let mut touched = Vec::new();
        for plot in (lo[1]..=hi[1]).flat_map(|y| (lo[0]..=hi[0]).map(move |x| [x, y])) {
            if let Some(stood) = index.get_mut(&(Seat::Sector(sector), plot)) {
                stood.version += 1;
                touched.push((plot, *stood));
            }
        }
        for (plot, stood) in &touched {
            room.keep(
                &volume_key(sector, *plot),
                stood.wire(*plot).encode_to_vec(),
            );
        }
        touched
    }

    fn open(&mut self, payload: &[u8], who: &Who, room: &mut dyn Room) {
        let Ok(open) = wire::Open::decode(payload) else {
            return room.refuse("message");
        };
        let sector = Seat::from_wire(open.seat.as_ref()).and_then(Seat::sector);
        let (Some(sector), true) = (sector, open.u.is_finite() && open.v.is_finite()) else {
            return room.refuse("message");
        };
        let seat = Seat::Sector(sector);
        let point = SurfacePoint::new(sector, open.u, open.v);
        let plot = seat::plot_of(point);
        if let Some(stood) = self.index(room).get(&(seat, plot)).copied() {
            // It stands already: the asker is told where, and nobody else.
            return tell_opened(room, seat, plot, stood, &[who.session]);
        }
        let stand = match room.ground().map(|ground| seat::survey(ground, point)) {
            None => return room.refuse("ground"),
            Some(Err(why)) => return room.refuse(why.code()),
            Some(Ok(stand)) => stand,
        };
        let stood = Stood { stand, version: 0 };
        room.keep(&volume_key(sector, plot), stood.wire(plot).encode_to_vec());
        self.index(room).insert((seat, plot), stood);
        let to = near(room, sector, &[(plot, stood)], TELL_M);
        tell_opened(room, seat, plot, stood, &to);
    }

    fn change(&mut self, payload: &[u8], who: &Who, room: &mut dyn Room) {
        let Ok(change) = wire::Change::decode(payload) else {
            return room.refuse("message");
        };
        let sector = Seat::from_wire(change.seat.as_ref()).and_then(Seat::sector);
        let gestures: Option<Vec<Gesture>> = change
            .gestures
            .iter()
            .map(seat::gesture_from_wire)
            .collect();
        let (Some(sector), Some(gestures)) = (sector, gestures) else {
            return room.refuse("message");
        };
        let within = |gesture: &Gesture| {
            let size = gesture.span().size();
            (0..3).all(|axis| size[axis] <= REACH[axis])
        };
        if gestures.len() > GESTURES || !gestures.iter().all(within) {
            return room.refuse("reach");
        }
        let seat = Seat::Sector(sector);
        let spans: Vec<Span> = gestures.iter().map(|gesture| gesture.span()).collect();
        let mut volumes = self.load(room, sector, &spans);
        let reach = spans
            .iter()
            .copied()
            .reduce(|a, b| a.with(b.min).with(b.max));
        // What is kept to take it back is what volumes hold of the change.
        let Some(span) = reach.and_then(|reach| volumes.held(reach)) else {
            return room.refuse("volume");
        };
        if span.size().iter().map(|&n| u64::from(n)).product::<u64>() > seat::CHANGE_CELLS {
            return room.refuse("reach");
        }
        let before = volumes.cells(span);
        let changed = gestures
            .iter()
            .filter_map(|&gesture| volumes.apply(gesture))
            .reduce(|a, b| a.with(b.min).with(b.max));
        let Some(changed) = changed else {
            return;
        };
        let touched = self.save(room, sector, &volumes, changed);
        let history = self.histories.entry(who.session).or_default();
        history.undone.clear();
        history.did(Kept::Change {
            seat,
            span,
            gestures,
            before: pack(&before),
        });
        let changed = wire::Changed {
            session: who.session,
            seat: Some(seat.wire()),
            gestures: change.gestures,
            stood: touched
                .iter()
                .map(|(plot, stood)| stood.wire(*plot))
                .collect(),
        };
        let to = near(room, sector, &touched, TELL_M);
        room.tell(wire::CHANGED, changed.encode_to_vec(), &to);
    }

    fn close(&mut self, payload: &[u8], who: &Who, room: &mut dyn Room) {
        let Ok(close) = wire::Close::decode(payload) else {
            return room.refuse("message");
        };
        let Some(seat) = Seat::from_wire(close.seat.as_ref()) else {
            return room.refuse("message");
        };
        let Some(kept) = self.shut(seat, [close.plot_x, close.plot_y], room) else {
            return room.refuse("volume");
        };
        let history = self.histories.entry(who.session).or_default();
        history.undone.clear();
        history.did(kept);
    }

    /// Takes the volume over a plot out of the world, and tells whoever is
    /// near that it is gone. What takes that back, where a volume stood.
    fn shut(&mut self, seat: Seat, plot: [i32; 2], room: &mut dyn Room) -> Option<Kept> {
        let sector = seat.sector()?;
        let stood = self.index(room).remove(&(seat, plot))?;
        let chunks = room.scan(&chunks_key(sector, plot));
        for (key, _) in &chunks {
            room.forget(key);
        }
        room.forget(&volume_key(sector, plot));
        let gone = wire::Held {
            seat: Some(seat.wire()),
            plot_x: plot[0],
            plot_y: plot[1],
            version: stood.version,
        };
        let seen = wire::Seen {
            volumes: Vec::new(),
            gone: vec![gone],
        };
        let to = near(room, sector, &[(plot, stood)], TELL_M);
        room.tell(wire::SEEN, seen.encode_to_vec(), &to);
        Some(Kept::Closed {
            seat,
            plot,
            stood,
            chunks,
        })
    }

    /// Stands a closed volume again where it stood, as it was, and shows it
    /// whole to whoever is near. False where another stands since.
    fn stand_again(
        &mut self,
        seat: Seat,
        plot: [i32; 2],
        stood: Stood,
        chunks: &[(Vec<u8>, Vec<u8>)],
        room: &mut dyn Room,
    ) -> bool {
        let Some(sector) = seat.sector() else {
            return false;
        };
        if self.index(room).contains_key(&(seat, plot)) {
            return false;
        }
        // One more than it was, so a client that still holds it asks again.
        let stood = Stood {
            version: stood.version + 1,
            ..stood
        };
        room.keep(&volume_key(sector, plot), stood.wire(plot).encode_to_vec());
        for (key, cells) in chunks {
            room.keep(key, cells.clone());
        }
        self.index(room).insert((seat, plot), stood);
        let to = near(room, sector, &[(plot, stood)], TELL_M);
        show(room, sector, plot, stood, &to);
        true
    }

    /// Takes back the last change of a session, or puts back the last it
    /// took back. A cell someone else changed since stays as they left it:
    /// taking back restores a cell that is still as the change made it, and
    /// putting back changes one that is still as it was before. A volume
    /// closed stands again where no other stands since, and putting that
    /// back closes it as it is then.
    fn restore(&mut self, back: bool, who: &Who, room: &mut dyn Room) {
        let history = self.histories.entry(who.session).or_default();
        let kept = match back {
            true => history.done.pop(),
            false => history.undone.pop(),
        };
        let kept = match kept {
            None => return room.refuse("none"),
            Some(Kept::Closed {
                seat,
                plot,
                stood,
                chunks,
            }) => {
                let kept = match back {
                    true => self
                        .stand_again(seat, plot, stood, &chunks, room)
                        .then_some(Kept::Closed {
                            seat,
                            plot,
                            stood,
                            chunks,
                        }),
                    false => self.shut(seat, plot, room),
                };
                let history = self.histories.entry(who.session).or_default();
                match (kept, back) {
                    (Some(kept), true) => history.undone.push(kept),
                    (Some(kept), false) => history.did(kept),
                    (None, _) => {}
                }
                return;
            }
            Some(kept @ Kept::Change { .. }) => kept,
        };
        let Kept::Change {
            seat,
            span,
            gestures,
            before: packed,
        } = &kept
        else {
            return;
        };
        let (seat, span) = (*seat, *span);
        let count = span.cells().count();
        let (Some(sector), Some(before)) = (seat.sector(), unpack(packed, count)) else {
            return room.refuse("none");
        };
        let mut volumes = self.load(room, sector, &[span]);
        let now = volumes.cells(span);
        let mut made = volumes.clone();
        made.restore(span, &before);
        for &gesture in gestures {
            made.apply(gesture);
        }
        let after = made.cells(span);
        let (from, to) = match back {
            true => (&after, &before),
            false => (&before, &after),
        };
        let next: Vec<Cell> = (0..count)
            .map(|at| if now[at] == from[at] { to[at] } else { now[at] })
            .collect();
        let touched = match volumes.restore(span, &next) {
            Some(changed) => self.save(room, sector, &volumes, changed),
            None => Vec::new(),
        };
        let history = self.histories.entry(who.session).or_default();
        match back {
            true => history.undone.push(kept),
            false => history.done.push(kept),
        }
        if touched.is_empty() {
            return;
        }
        let restored = wire::Restored {
            session: who.session,
            seat: Some(seat.wire()),
            x0: span.min[0],
            y0: span.min[1],
            z0: span.min[2],
            x1: span.max[0],
            y1: span.max[1],
            z1: span.max[2],
            cells: pack(&next),
            stood: touched
                .iter()
                .map(|(plot, stood)| stood.wire(*plot))
                .collect(),
        };
        let to = near(room, sector, &touched, TELL_M);
        room.tell(wire::RESTORED, restored.encode_to_vec(), &to);
    }

    /// Answers what a client holds with what it lacks: every volume near its
    /// body that it has not, or has at another version, whole, the nearest
    /// first, and those it holds that stand no more.
    fn look(&mut self, payload: &[u8], who: &Who, room: &mut dyn Room) {
        let Ok(look) = wire::Look::decode(payload) else {
            return room.refuse("message");
        };
        let measure = room.measure();
        let held: BTreeMap<Plot, u64> = look
            .held
            .iter()
            .filter_map(|held| {
                let seat = Seat::from_wire(held.seat.as_ref())?;
                Some(((seat, [held.plot_x, held.plot_y]), held.version))
            })
            .collect();
        let index = self.index(room);
        let gone: Vec<wire::Held> = look
            .held
            .iter()
            .filter(|held| {
                let seat = Seat::from_wire(held.seat.as_ref());
                seat.is_none_or(|seat| !index.contains_key(&(seat, [held.plot_x, held.plot_y])))
            })
            .cloned()
            .collect();
        let from = who
            .stance
            .as_ref()
            .and_then(|stance| measure.position(stance));
        let mut lacking: Vec<(f64, Sector, [i32; 2], Stood)> = index
            .iter()
            .filter_map(|(&(seat, plot), &stood)| {
                let sector = seat.sector()?;
                let away_m = seat::away_m(measure.sphere, sector, plot, stood.stand, from?);
                let lacks = held.get(&(seat, plot)) != Some(&stood.version);
                (away_m <= HOLD_M && lacks).then_some((away_m, sector, plot, stood))
            })
            .collect();
        lacking.sort_by(|a, b| a.0.total_cmp(&b.0));
        let to = [who.session];
        if !gone.is_empty() {
            let seen = wire::Seen {
                volumes: Vec::new(),
                gone,
            };
            room.tell(wire::SEEN, seen.encode_to_vec(), &to);
        }
        let mut budget = LOOK_BYTES;
        for (_, sector, plot, stood) in lacking {
            budget = budget.saturating_sub(show(room, sector, plot, stood, &to));
            if budget == 0 {
                break;
            }
        }
    }
}

/// Shows a volume whole, in as many messages as its chunks take. How many
/// bytes of chunks went.
fn show(room: &mut dyn Room, sector: Sector, plot: [i32; 2], stood: Stood, to: &[u32]) -> usize {
    let mut volume = wire::Volume {
        seat: Some(Seat::Sector(sector).wire()),
        stood: Some(stood.wire(plot)),
        chunks: Vec::new(),
    };
    let (mut bytes, mut sent, mut all) = (0, false, 0);
    for (key, cells) in room.scan(&chunks_key(sector, plot)) {
        let at = |from: usize| number(key.get(from..from + 4)?);
        let (Some(x), Some(y), Some(z)) = (at(10), at(14), at(18)) else {
            continue;
        };
        bytes += cells.len();
        volume.chunks.push(wire::Chunk { x, y, z, cells });
        if bytes > SEEN_BYTES {
            all += bytes;
            tell_seen(room, &mut volume, to);
            (bytes, sent) = (0, true);
        }
    }
    if !sent || !volume.chunks.is_empty() {
        all += bytes;
        tell_seen(room, &mut volume, to);
    }
    all
}

/// Says a volume, or the part of it gathered so far, and starts the next
/// part with no chunk.
fn tell_seen(room: &mut dyn Room, volume: &mut wire::Volume, to: &[u32]) {
    let seen = wire::Seen {
        volumes: vec![volume.clone()],
        gone: Vec::new(),
    };
    room.tell(wire::SEEN, seen.encode_to_vec(), to);
    volume.chunks.clear();
}

fn tell_opened(room: &mut dyn Room, seat: Seat, plot: [i32; 2], stood: Stood, to: &[u32]) {
    let opened = wire::Opened {
        seat: Some(seat.wire()),
        stood: Some(stood.wire(plot)),
    };
    room.tell(wire::OPENED, opened.encode_to_vec(), to);
}

/// The sessions whose bodies are within reach of any of some volumes.
fn near(room: &dyn Room, sector: Sector, volumes: &[([i32; 2], Stood)], reach_m: f64) -> Vec<u32> {
    let measure = room.measure();
    let within = |at: [f64; 3]| {
        volumes.iter().any(|(plot, stood)| {
            seat::away_m(measure.sphere, sector, *plot, stood.stand, at) <= reach_m
        })
    };
    room.sessions()
        .iter()
        .filter(|who| {
            let at = who
                .stance
                .as_ref()
                .and_then(|stance| measure.position(stance));
            at.is_some_and(within)
        })
        .map(|who| who.session)
        .collect()
}

impl Plugin for Cells {
    fn name(&self) -> &'static str {
        wire::OWNER
    }

    fn version(&self) -> u32 {
        VERSION
    }

    /// Changing the cells is a builder's and an admin's. Anyone looks.
    fn ops(&self) -> Vec<Op> {
        let builder = [
            wire::OPEN,
            wire::CHANGE,
            wire::CLOSE,
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
            wire::OPEN => self.open(payload, who, room),
            wire::CHANGE => self.change(payload, who, room),
            wire::CLOSE => self.close(payload, who, room),
            wire::TAKE_BACK => self.restore(true, who, room),
            wire::PUT_BACK => self.restore(false, who, room),
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
