//! The build plugin's client half (DECISIONS 106): how a hand arrives at
//! gestures. A tool in hand, a stroke from a click and a drag, a platform
//! where the body stands, the key that turns a stroke and the chord that
//! takes one back. While it builds it shows where cells are: the volume the
//! body is in, and the slab a platform would lay. The cells are the core's:
//! this half reads them and asks for gestures through its host, and holds
//! none.
//!
//! Over the seam, JSON tagged by `type`. What a front end or an agent asks:
//!
//! ```json
//! {"type":"build.take","tool":"create"}
//! {"type":"build.paint","paint":4}
//! {"type":"build.platform","side":32}
//! {"type":"build.lay","base":"deck"}
//! {"type":"build.close"}
//! {"type":"build.undo"}
//! {"type":"build.redo"}
//! {"type":"build.state"}
//! ```
//!
//! And what it hears:
//!
//! ```json
//! {"type":"build.hand","tool":"create","paint":4,"platform":32}
//! {"type":"build.over","volume":true}
//! {"type":"build.refused","reason":"sea"}
//! {"type":"build.history","undo":true,"redo":false}
//! ```

mod stroke;

use build_world::Platform;
use client::{Aim, Feet, Guide, Host, KeyAsk, Plugin, Turn};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use topology::{BLOCK_M, Sector, SurfacePoint};
use voxel::Span;

use stroke::Stroke;

/// The plugin's name, its version and the sides a platform can have are said
/// once, by its world half (DECISIONS 101).
pub use build_world::{NAME, PLATFORMS, VERSION};

/// What a stroke in a volume does: fill air with the paint, empty cells, or
/// repaint what is solid. One drag is one stroke, however many cells it covers.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tool {
    Create,
    Delete,
    Paint,
}

/// What carries the slab of a platform down to the ground: a deck's pillars
/// at the corners of every bay, a solid block of every column filled, or
/// nothing, a slab that floats where it is laid.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Base {
    #[default]
    Deck,
    Solid,
    Floating,
}

impl Base {
    fn kind(self) -> build_world::Base {
        match self {
            Base::Deck => build_world::Base::Deck,
            Base::Solid => build_world::Base::Solid,
            Base::Floating => build_world::Base::Floating,
        }
    }
}

/// Why a tool was not handed over, or no platform laid where the body
/// stands. A front end says it in its own words.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Refusal {
    /// Volumes stand on the planet.
    Moon,
    /// The ground here is under the sea.
    Sea,
    /// The plot is on the edge of a sector: a volume stays inside one.
    Seam,
    /// The feet are over the top of the volume: it holds what is built from
    /// its lowest ground to a height over its highest, and no further.
    High,
    /// Building is a builder's and an admin's, and the world named this
    /// session a lower level.
    Level,
    /// This world is shaped by a field, and its server does not hold the
    /// field to seat a volume by.
    Field,
    /// No volume stands under the body: there is none to close.
    Empty,
}

impl From<client::Refusal> for Refusal {
    fn from(refusal: client::Refusal) -> Refusal {
        match refusal {
            client::Refusal::Level => Refusal::Level,
            client::Refusal::Field => Refusal::Field,
            client::Refusal::Sea => Refusal::Sea,
            client::Refusal::Seam => Refusal::Seam,
        }
    }
}

/// `build.take`: build with a tool, or stop building with `null`. It builds
/// nothing: a stroke starts on what is built, and a platform is the first of
/// it.
pub const TAKE: &str = "take";
/// `build.paint`: the paint the next stroke lays, an index into the world's
/// palette.
pub const PAINT: &str = "paint";
/// `build.platform`: the side of the platform laid next, in cells. The size
/// on offer nearest to it is taken.
pub const PLATFORM: &str = "platform";
/// `build.lay`: lays a platform where the body stands, opening the volume of
/// its plot where none stands: a slab as high as the higher of the highest
/// ground under it and the feet, on a base. A deck when the base is left out.
pub const LAY: &str = "lay";
/// `build.close`: closes the volume the body is over. What was built in it
/// goes with it and its plot is nature again: one change, taken back as one.
pub const CLOSE: &str = "close";
/// `build.undo`: takes back the last change that landed.
pub const UNDO: &str = "undo";
/// `build.redo`: puts back the last change taken back.
pub const REDO: &str = "redo";
/// `build.state`: asks what is in hand, what the body is over and what there
/// is to take back. It changes nothing, and is answered with [`HAND`],
/// [`OVER`] and [`HISTORY`].
pub const STATE: &str = "state";

/// `build.hand`: the tool in hand, `null` when not building, the paint it
/// lays and the side of the platform laid next, whenever one changes.
pub const HAND: &str = "hand";
/// `build.over`: whether a volume stands under the body, whenever that
/// changes while building.
pub const OVER: &str = "over";
/// `build.refused`: a tool, a platform or the closing of a volume was asked
/// for and not given.
pub const REFUSED: &str = "refused";
/// `build.history`: whether there is a change to take back and one to put
/// back, whenever that changes.
pub const HISTORY: &str = "history";

#[derive(Deserialize)]
struct Take {
    tool: Option<Tool>,
}

#[derive(Deserialize)]
struct Paint {
    paint: u8,
}

#[derive(Deserialize)]
struct Side {
    side: u32,
}

#[derive(Deserialize)]
struct Lay {
    #[serde(default)]
    base: Base,
}

#[derive(Serialize)]
struct Hand {
    tool: Option<Tool>,
    paint: u8,
    platform: u32,
}

#[derive(Serialize)]
struct Over {
    volume: bool,
}

#[derive(Serialize)]
struct Refused {
    reason: Refusal,
}

#[derive(Serialize)]
struct History {
    undo: bool,
    redo: bool,
}

/// The keys building asks for, by the names it hears them by.
mod key {
    /// Start building with the last tool, or stop.
    pub const BUILD: &str = "build";
    /// Take a tool, which starts building if it was not.
    pub const CREATE: &str = "create";
    pub const DELETE: &str = "delete";
    pub const PAINT: &str = "paint";
    /// Each time it goes down, the stroke turns to the next of the three
    /// layers through its start: on its side, then standing one way, then
    /// the other.
    pub const TURN: &str = "turn";
    /// Drop the stroke being drawn, or else stop building.
    pub const CANCEL: &str = "cancel";
    /// Take back the last change, or put it back.
    pub const UNDO: &str = "undo";
    pub const REDO: &str = "redo";
}

/// The paints a hand can pick: as many as the world's palette holds.
const PAINTS: u8 = client::PALETTE.len() as u8;
/// Rows of the ground under a platform read in one turn: a platform of 64
/// reads 65 rows of 65 corners, each a sample of the ground in full detail.
const GROUND_ROWS_PER_TURN: usize = 13;

/// The ground under the square of a platform, read a few rows a turn: the
/// height of each corner of each column, in blocks, row by row.
#[derive(Clone)]
struct Survey {
    sector: Sector,
    /// The square, its top left unsaid.
    square: Platform,
    /// The cells the volume of its plot holds, or would hold once opened.
    /// None where no volume can stand, and no platform be laid.
    room: Option<Span>,
    corners: Vec<f64>,
}

impl Survey {
    /// The square of `2^bits` columns a side that a body's feet are over.
    fn square(feet: Feet, bits: u32) -> Platform {
        let (x, y) = (feet.point.u.floor() as i32, feet.point.v.floor() as i32);
        Platform::over(x, y, bits, 0)
    }

    /// The square of `2^bits` columns a side that a body's feet are over,
    /// with none of its ground read.
    fn under(feet: Feet, bits: u32, host: &Host<'_>) -> Survey {
        let square = Survey::square(feet, bits);
        let across = square.side as usize + 1;
        Survey {
            sector: feet.point.sector,
            square,
            room: host.room(feet.point).ok(),
            corners: Vec::with_capacity(across * across),
        }
    }

    /// Whether it is of the square of `2^bits` columns a side that a body's
    /// feet are over.
    fn is_under(&self, feet: Feet, bits: u32) -> bool {
        self.sector == feet.point.sector && self.square == Survey::square(feet, bits)
    }

    fn across(&self) -> usize {
        self.square.side as usize + 1
    }

    /// Whether every corner of every column is read.
    fn read(&self) -> bool {
        self.corners.len() >= self.across() * self.across()
    }

    /// Reads up to `rows` more rows of the ground. True once all is read.
    fn more(&mut self, host: &Host<'_>, rows: usize) -> bool {
        let across = self.across();
        let [x0, y0] = self.square.corner;
        let read = self.corners.len() / across;
        for row in read..(read.saturating_add(rows)).min(across) {
            self.corners.extend((0..across).map(|dx| {
                let (u, v) = (x0 + dx as i32, y0 + row as i32);
                host.ground(SurfacePoint::new(self.sector, f64::from(u), f64::from(v)))
            }));
        }
        self.read()
    }

    /// The platform a body with its feet `feet` cells high is given on this
    /// square: its top is the higher of the highest ground under it and the
    /// feet (DECISIONS 105).
    fn platform(&self, feet: i32) -> Platform {
        let highest = self.corners.iter().copied().fold(f64::MIN, f64::max);
        Platform {
            top: (highest.ceil() as i32).max(feet),
            ..self.square
        }
    }

    /// The lowest ground at the foot of a column, in cells: a base reaches
    /// it, so no ground shows under it.
    fn foot(&self, x: i32, y: i32) -> i32 {
        let across = self.across();
        let [x0, y0] = self.square.corner;
        let (dx, dy) = ((x - x0) as usize, (y - y0) as usize);
        let at = |dx: usize, dy: usize| self.corners[dy * across + dx];
        let lowest = at(dx, dy)
            .min(at(dx + 1, dy))
            .min(at(dx, dy + 1))
            .min(at(dx + 1, dy + 1));
        lowest.floor() as i32
    }
}

/// How high a body's feet stand, in cells. To the nearest, so a body on a
/// slab is given the next one level with it, whichever side of the cell's
/// edge its feet are.
fn level(feet: Feet) -> i32 {
    (feet.height_m / BLOCK_M).round() as i32
}

/// A platform asked for, whose ground is still being read.
struct Laying {
    survey: Survey,
    base: Base,
    paint: u8,
    /// How high the feet of the one who asked stand, in cells: the slab
    /// stands no lower.
    feet: i32,
}

/// The tool in hand and what it is drawing.
pub struct Build {
    tool: Option<Tool>,
    /// The tool taken last, which starting to build again takes.
    last_tool: Tool,
    paint: u8,
    /// The side of the next platform, in cells, as a power of two.
    platform_bits: u32,
    aim: Option<Aim>,
    stroke: Option<Stroke>,
    using: bool,
    /// Whether the key that turns a stroke was down last turn: a stroke
    /// turns as it goes down.
    turning: bool,
    /// How many times the stroke being drawn, or the next one, was turned.
    turned: usize,
    /// The platform asked for whose ground is still being read.
    laying: Option<Laying>,
    /// The ground under the square the body is over, read while a tool is
    /// in hand: what the slab of the next platform is shown by.
    survey: Option<Survey>,
    /// Whether laying a slab would make a cell, as last worked out: the
    /// slab, and the count of the cells' changes it was read at.
    lays: Option<(Span, u64, bool)>,
    /// Whether a volume stood under the body, as last said.
    over: bool,
    /// Whether there was a change to take back and one to put back, as last
    /// said.
    history: (bool, bool),
}

impl Default for Build {
    fn default() -> Self {
        Build {
            tool: None,
            last_tool: Tool::Create,
            paint: 0,
            platform_bits: 4,
            aim: None,
            stroke: None,
            using: false,
            turning: false,
            turned: 0,
            laying: None,
            survey: None,
            lays: None,
            over: false,
            history: (false, false),
        }
    }
}

/// The plugin as a shell plugs it in.
pub fn plugin() -> Box<dyn Plugin> {
    Box::new(Build::default())
}

impl Build {
    /// The side of the platform that is laid next, in cells.
    fn platform(&self) -> u32 {
        1 << self.platform_bits
    }

    /// Picks the side of the platform that is laid next: the one on offer
    /// nearest to `side` cells.
    fn set_platform(&mut self, side: u32) {
        let nearest = PLATFORMS.into_iter().min_by_key(|on| on.abs_diff(side));
        self.platform_bits = nearest.unwrap_or(PLATFORMS[0]).trailing_zeros();
    }

    fn set_paint(&mut self, paint: u8) {
        self.paint = paint.min(PAINTS - 1);
    }

    /// Takes a tool or puts it down. A stroke half drawn is dropped. With a
    /// tool in hand the pointer is this plugin's, and the body flies through
    /// what it builds.
    fn take(&mut self, tool: Option<Tool>, host: &mut Host<'_>) {
        if tool.is_some() && !host.may_change() {
            return refuse(host, Refusal::Level);
        }
        self.tool = tool;
        if let Some(tool) = tool {
            self.last_tool = tool;
        }
        self.stroke = None;
        self.turned = 0;
        self.aim = None;
        host.point(tool.is_some());
        host.pass(tool.is_some());
        if tool.is_none() {
            host.preview(None);
            host.guide(&[]);
            self.survey = None;
            self.lays = None;
        }
        self.say_hand(host);
    }

    /// Drops the stroke being drawn, if there is one. True when there was.
    fn cancel(&mut self) -> bool {
        self.turned = 0;
        self.stroke.take().is_some()
    }

    fn undo(&mut self, host: &mut Host<'_>) {
        self.stroke = None;
        if let Err(refusal) = host.take_back() {
            refuse(host, refusal.into());
        }
        self.say_history(host);
    }

    fn redo(&mut self, host: &mut Host<'_>) {
        self.stroke = None;
        if let Err(refusal) = host.put_back() {
            refuse(host, refusal.into());
        }
        self.say_history(host);
    }

    fn say_hand(&self, host: &mut Host<'_>) {
        host.emit(
            HAND,
            &Hand {
                tool: self.tool,
                paint: self.paint,
                platform: self.platform(),
            },
        );
    }

    /// Whether a volume stands under the body.
    fn is_over(host: &Host<'_>) -> bool {
        let feet = host.feet();
        !feet.moon && host.cells().covers(feet.point)
    }

    /// Says whether a volume stands under the body, when that changed.
    fn say_over(&mut self, host: &mut Host<'_>) {
        let volume = Build::is_over(host);
        if volume != self.over {
            self.over = volume;
            host.emit(OVER, &Over { volume });
        }
    }

    /// Closes the volume the body is over.
    fn close(&mut self, host: &mut Host<'_>) -> Result<(), Refusal> {
        if !host.may_change() {
            return Err(Refusal::Level);
        }
        let feet = host.feet();
        if feet.moon {
            return Err(Refusal::Moon);
        }
        // What was half done in it goes with it.
        self.cancel();
        self.laying = None;
        match host.close(feet.point)? {
            true => Ok(()),
            false => Err(Refusal::Empty),
        }
    }

    /// Shows where cells are while a tool is in hand: the volume the body is
    /// in, as room to build in, and the slab a platform would lay where it
    /// stands, in the paint in hand, wherever laying one would make a cell.
    /// The ground under the slab is read `rows` rows a turn, and the slab
    /// shows once all of it is.
    fn guide(&mut self, host: &mut Host<'_>, rows: usize) {
        if self.tool.is_none() {
            return;
        }
        let feet = host.feet();
        if feet.moon {
            self.survey = None;
            return host.guide(&[]);
        }
        let seat = feet.point.sector.into();
        let room = host.cells().bounds_over(feet.point).map(|span| Guide {
            seat,
            span,
            paint: None,
        });
        let slab = self.slab(feet, host, rows).map(|span| Guide {
            seat,
            span,
            paint: Some(self.paint),
        });
        let guides: Vec<Guide> = room.into_iter().chain(slab).collect();
        host.guide(&guides);
    }

    /// The slab of the platform a body would be given where it stands, once
    /// the ground under it is read, `rows` more rows of it now, when laying
    /// it would make a cell.
    fn slab(&mut self, feet: Feet, host: &Host<'_>, rows: usize) -> Option<Span> {
        let bits = self.platform_bits;
        if !matches!(&self.survey, Some(held) if held.is_under(feet, bits)) {
            self.survey = Some(Survey::under(feet, bits, host));
        }
        let survey = self.survey.as_mut()?;
        let room = survey.room?;
        // A platform being laid has the turn's reading of the ground.
        if !survey.read() && (self.laying.is_some() || !survey.more(host, rows)) {
            return None;
        }
        let feet = level(feet);
        if feet > room.max[2] + 1 {
            return None;
        }
        let slab = survey.platform(feet).slab();
        let (cells, seat) = (host.cells(), survey.sector);
        let revision = cells.revision();
        let lays = match self.lays {
            Some((span, at, lays)) if span == slab && at == revision => lays,
            _ => slab.cells().any(|cell| cells.cell(seat, cell).is_air()),
        };
        self.lays = Some((slab, revision, lays));
        lays.then_some(slab)
    }

    /// Says whether there is a change to take back or to put back, when that
    /// changed.
    fn say_history(&mut self, host: &mut Host<'_>) {
        let history = host.cells().history();
        if history != self.history {
            self.history = history;
            let (undo, redo) = history;
            host.emit(HISTORY, &History { undo, redo });
        }
    }

    /// Asks for a platform where the body stands.
    fn lay(&mut self, base: Base, host: &mut Host<'_>) -> Result<(), Refusal> {
        self.lay_at(host.feet(), base, host)
    }

    /// Asks for a platform where a body's feet are, opening the volume of
    /// its plot if none stands there: a slab of the side picked, in the
    /// paint in hand, on `base` down to the ground. Its top is the higher of
    /// the highest ground under it and the feet (DECISIONS 105): feet on the
    /// ground, the ground decides, as it does on a slope; a body in the air,
    /// or standing on what is built, is given the slab where it stands. The
    /// ground under it is read a few rows a turn and the platform lands when
    /// all of it is read, one change like any other, taken back as one. A
    /// platform asked for while another is being read takes its place.
    fn lay_at(&mut self, feet: Feet, base: Base, host: &mut Host<'_>) -> Result<(), Refusal> {
        if !host.may_change() {
            return Err(Refusal::Level);
        }
        if feet.moon {
            return Err(Refusal::Moon);
        }
        host.open(feet.point)?;
        // A slab is the cell under its top, and a volume ends where it ends.
        let ceiling = host
            .cells()
            .bounds_over(feet.point)
            .map_or(i32::MAX, |held| held.max[2] + 1);
        if level(feet) > ceiling {
            return Err(Refusal::High);
        }
        // The ground the slab was shown by is read already, or partly.
        let bits = self.platform_bits;
        let survey = match &self.survey {
            Some(held) if held.is_under(feet, bits) => held.clone(),
            _ => Survey::under(feet, bits, host),
        };
        self.laying = Some(Laying {
            survey,
            base,
            paint: self.paint,
            feet: level(feet),
        });
        Ok(())
    }

    /// Reads up to `rows` more rows of the ground under the platform being
    /// laid, and lays it once every corner of every column is read: the
    /// gestures it comes to, as one change, and the body lifted onto its
    /// slab. The height of its top, in cells, when it landed.
    fn read_ground(&mut self, host: &mut Host<'_>, rows: usize) -> Option<i32> {
        if !self.laying.as_mut()?.survey.more(host, rows) {
            return None;
        }
        let Laying {
            survey,
            base,
            paint,
            feet,
        } = self.laying.take()?;
        let platform = survey.platform(feet);
        let gestures = platform.gestures(base.kind(), |x, y| survey.foot(x, y), paint);
        match host.apply(survey.sector, &gestures) {
            Ok(_) => host.lift(f64::from(platform.top) * BLOCK_M),
            Err(refusal) => refuse(host, refusal.into()),
        }
        Some(platform.top)
    }
}

fn refuse(host: &mut Host<'_>, reason: Refusal) {
    host.emit(REFUSED, &Refused { reason });
}

impl Plugin for Build {
    fn name(&self) -> &'static str {
        NAME
    }

    fn version(&self) -> u32 {
        VERSION
    }

    fn command(&mut self, kind: &str, body: Value, host: &mut Host<'_>) {
        fn read<T: for<'de> Deserialize<'de>>(body: Value, host: &mut Host<'_>) -> Option<T> {
            serde_json::from_value(body)
                .map_err(|error| host.reject(error.to_string()))
                .ok()
        }
        match kind {
            TAKE => {
                if let Some(Take { tool }) = read(body, host) {
                    self.take(tool, host);
                }
            }
            PAINT => {
                if let Some(Paint { paint }) = read(body, host) {
                    self.set_paint(paint);
                    self.say_hand(host);
                }
            }
            PLATFORM => {
                if let Some(Side { side }) = read(body, host) {
                    self.set_platform(side);
                    self.say_hand(host);
                }
            }
            LAY => {
                if let Some(Lay { base }) = read(body, host)
                    && let Err(reason) = self.lay(base, host)
                {
                    refuse(host, reason);
                }
            }
            CLOSE => {
                if let Err(reason) = self.close(host) {
                    refuse(host, reason);
                }
            }
            UNDO => self.undo(host),
            REDO => self.redo(host),
            STATE => {
                self.say_hand(host);
                self.over = Build::is_over(host);
                host.emit(OVER, &Over { volume: self.over });
                let (undo, redo) = host.cells().history();
                self.history = (undo, redo);
                host.emit(HISTORY, &History { undo, redo });
            }
            _ => host.reject(format!("`{kind}` is no command of build")),
        }
    }

    fn keys(&self) -> Vec<KeyAsk> {
        vec![
            KeyAsk::pressed(key::BUILD, "KeyB"),
            KeyAsk::pressed(key::CREATE, "Digit1"),
            KeyAsk::pressed(key::DELETE, "Digit2"),
            KeyAsk::pressed(key::PAINT, "Digit3"),
            KeyAsk::pressed(key::CANCEL, "Escape"),
            KeyAsk::held(key::TURN, "AltLeft"),
            KeyAsk::held(key::TURN, "AltRight"),
            KeyAsk::command(key::UNDO, "KeyZ", false),
            KeyAsk::command(key::REDO, "KeyZ", true),
            KeyAsk::command(key::REDO, "KeyY", false),
        ]
    }

    fn key(&mut self, name: &str, host: &mut Host<'_>) {
        match name {
            key::BUILD => {
                let tool = match self.tool {
                    Some(_) => None,
                    None => Some(self.last_tool),
                };
                self.take(tool, host);
            }
            key::CREATE => self.take(Some(Tool::Create), host),
            key::DELETE => self.take(Some(Tool::Delete), host),
            key::PAINT => self.take(Some(Tool::Paint), host),
            // A stroke half drawn goes first, then the tool itself.
            key::CANCEL => {
                if !self.cancel() && self.tool.is_some() {
                    self.take(None, host);
                }
            }
            key::UNDO => self.undo(host),
            key::REDO => self.redo(host),
            _ => {}
        }
    }

    /// One turn of the tool: what the line of sight through the pointer
    /// meets, the stroke while the button is held, and the gesture when it
    /// is let go. Then a few more rows of the ground under a platform asked
    /// for, and where cells are, for a hand to see.
    fn turn(&mut self, turn: &Turn<'_>, host: &mut Host<'_>) {
        if turn.interrupted {
            self.cancel();
        }
        self.handle(turn, host);
        self.read_ground(host, GROUND_ROWS_PER_TURN);
        self.guide(host, GROUND_ROWS_PER_TURN);
        if self.tool.is_some() {
            self.say_over(host);
        }
        self.say_history(host);
    }

    fn busy(&self) -> bool {
        let reading = |survey: &Survey| survey.room.is_some() && !survey.read();
        self.laying.is_some() || self.survey.as_ref().is_some_and(reading)
    }

    fn settle(&mut self, host: &mut Host<'_>) {
        self.read_ground(host, usize::MAX);
        self.guide(host, usize::MAX);
        self.say_history(host);
    }

    fn rest(&mut self, host: &mut Host<'_>) {
        self.laying = None;
        self.using = false;
        self.over = false;
        if self.tool.is_some() {
            self.take(None, host);
        }
    }
}

#[cfg(test)]
mod tests;
