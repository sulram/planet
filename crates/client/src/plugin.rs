//! The host of plugins on the client: who is plugged in, which of them the
//! world has on, and what each is offered (DECISIONS 88, 93, 98, 108).
//!
//! A plugin's client half is a crate of its own that imports this one. The
//! core names no plugin: a shell plugs in what its version carries, the
//! world's statement says which are on, and a command, a frame from the
//! server, a key or a turn reaches a plugin by the name it gave.
//!
//! A plugin speaks to the core's systems through its [`Host`]: the ground,
//! the cells, the body. What a system holds is read freely. What changes it
//! is an op, and the host asks whether this session may before the system
//! sees it.

use glam::{DQuat, DVec3};
use serde::Serialize;
use serde_json::Value;
use topology::{QuadSphere, SurfacePoint};
use voxel::{Gesture, Span};
use worldgen::Generator;

use crate::cells::{self, Cells, Guide, Refusal, Seat, Sight};
use crate::controller::Controller;
use crate::input::{Chord, Input, Key, KeyAsk};
use crate::place::Pose;
use crate::seam::{Event, Level, PluginOn};
use crate::session::Session;

/// A plugin's client half.
pub trait Plugin {
    /// The name the wire's envelope and the world's statement say, and the
    /// one before the dot in its commands and events: `chat.say`.
    fn name(&self) -> &'static str;

    /// The version of its wire and of its seam. A world that speaks another
    /// leaves it off here.
    fn version(&self) -> u32;

    /// A command from a front end or an agent, its name without the
    /// plugin's: `say` for `chat.say`. `body` is the rest of its JSON.
    fn command(&mut self, kind: &str, body: Value, host: &mut Host<'_>);

    /// A message of this plugin from the world server.
    fn receive(&mut self, _kind: &str, _payload: &[u8], _host: &mut Host<'_>) {}

    /// The keys it asks for, by name: data, which each shell binds
    /// (DECISIONS 106). Asked once, as it is plugged in.
    fn keys(&self) -> Vec<KeyAsk> {
        Vec::new()
    }

    /// A key it asked for went down, by the name it gave it.
    fn key(&mut self, _name: &str, _host: &mut Host<'_>) {}

    /// Its turn in a frame, after the body moved and before the picture is
    /// made.
    fn turn(&mut self, _turn: &Turn<'_>, _host: &mut Host<'_>) {}

    /// Whether it owes the world work it spreads over turns. The world is
    /// not settled while a plugin is busy.
    fn busy(&self) -> bool {
        false
    }

    /// Does at once all the work it owes: for a picture that must show it.
    fn settle(&mut self, _host: &mut Host<'_>) {}

    /// The world it worked in switched it off, or is gone: it puts down what
    /// it had in hand.
    fn rest(&mut self, _host: &mut Host<'_>) {}
}

/// The eye a hand aims from: where the camera is, how it looks, and where
/// the pointer is on the view. What a plugin is handed in its turn: the
/// cells take a line of sight, and know no eye.
#[derive(Clone, Copy, Debug)]
pub struct Eye {
    pub position: DVec3,
    /// World from camera. The camera looks down its `-Z`, `+Y` is up.
    pub rotation: DQuat,
    /// Vertical field of view, radians.
    pub fov_y: f64,
    /// Width over height.
    pub aspect: f64,
    /// Fractions of the view from the top left.
    pub pointer: [f64; 2],
}

impl Eye {
    /// The pointer on the view in units of half its height, `y` up: a
    /// distance there is the same across and down.
    fn at(&self) -> [f64; 2] {
        let [x, y] = self.pointer;
        [(2.0 * x - 1.0) * self.aspect, 1.0 - 2.0 * y]
    }

    /// Where the eye is and which way the pointer looks from it, unit.
    pub fn sight(&self) -> (DVec3, DVec3) {
        let [x, y] = self.at();
        let tan = (self.fov_y / 2.0).tan();
        let look = DVec3::new(x * tan, y * tan, -1.0).normalize();
        (self.position, self.rotation * look)
    }
}

/// A plugin's turn in a frame: where the eye and the pointer are, and what
/// of the plugin's is held down.
pub struct Turn<'a> {
    /// The eye, and where the pointer is on its view.
    pub eye: Eye,
    /// Whether the pointer's button is down. False for a plugin that has
    /// not the pointer.
    pub using: bool,
    /// The keys this plugin asked to read held that are down, by name.
    pub keys: &'a [&'static str],
    /// Whether the shell let everything go since the last turn: what was
    /// half done is dropped, never landed.
    pub interrupted: bool,
}

impl Turn<'_> {
    /// Whether a key this plugin asked to read held is down.
    pub fn held(&self, name: &str) -> bool {
        self.keys.contains(&name)
    }
}

/// Where the feet of this client's own body are.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Feet {
    /// The column they are over.
    pub point: SurfacePoint,
    /// How high over the datum, metres.
    pub height_m: f64,
    /// Whether they are on the moon, where `point` names nothing.
    pub moon: bool,
}

/// What of the core a host lends a plugin for one call.
pub(crate) struct Lent<'a> {
    pub session: &'a mut Session,
    pub events: &'a mut Vec<Event>,
    pub generator: &'a Generator,
    pub cells: &'a mut Cells,
    pub controller: &'a mut Controller,
    pub level: Option<Level>,
    /// The plugin that has the pointer, while one does.
    pub pointer: &'a mut Option<&'static str>,
}

impl<'a> Lent<'a> {
    fn again(&mut self) -> Lent<'_> {
        Lent {
            session: self.session,
            events: self.events,
            generator: self.generator,
            cells: self.cells,
            controller: self.controller,
            level: self.level,
            pointer: self.pointer,
        }
    }

    pub(crate) fn host(self, plugin: &'static str) -> Host<'a> {
        Host { plugin, lent: self }
    }
}

/// What the core offers a plugin while it handles a command, a message, a
/// key or its turn.
pub struct Host<'a> {
    plugin: &'static str,
    lent: Lent<'a>,
}

impl Host<'_> {
    /// Asks the world for an op of this plugin: the payload rides the core's
    /// envelope under the plugin's name. Nothing goes out while offline.
    pub fn send(&mut self, kind: &str, payload: Vec<u8>) {
        self.lent.session.envelope(self.plugin, kind, payload);
    }

    /// Says an event of this plugin over the seam. `body` is an object, and
    /// its `type` becomes the plugin's name and `kind`: `chat.said`.
    pub fn emit<T: Serialize>(&mut self, kind: &str, body: &T) {
        match serde_json::to_value(body) {
            Ok(Value::Object(mut event)) => {
                event.insert("type".into(), format!("{}.{kind}", self.plugin).into());
                self.lent.events.push(Event::Plugin(Value::Object(event)));
            }
            _ => self.reject(format!("the event `{kind}` is no object")),
        }
    }

    /// Refuses what was asked. The message is for logs, not for people.
    pub fn reject(&mut self, message: String) {
        self.lent.events.push(Event::Rejected {
            message: format!("{}: {message}", self.plugin),
        });
    }

    /// A stance as a place: what a person reads out and `GoTo` takes. `None`
    /// for a stance that names nowhere on this body.
    pub fn place(&self, stance: &protocol::Stance) -> Option<String> {
        let grid = self.sphere().blocks();
        Pose::of_stance(grid, stance).map(|pose| pose.place(grid))
    }

    /// The body of the world: how an address is a place on it.
    pub fn sphere(&self) -> QuadSphere {
        self.lent.generator.sphere()
    }

    /// How high the ground stands at a column, in blocks: the ground in full
    /// detail, as a body stands on it.
    pub fn ground(&self, point: SurfacePoint) -> f64 {
        cells::ground(self.lent.generator, point)
    }

    /// Where the feet of this client's own body are.
    pub fn feet(&self) -> Feet {
        let controller = &self.lent.controller;
        Feet {
            point: controller.point(),
            height_m: controller.height_m(),
            moon: controller.on_moon(),
        }
    }

    /// Lifts the body onto a floor `floor_m` over the datum, when it stands
    /// lower.
    pub fn lift(&mut self, floor_m: f64) {
        self.lent.controller.lift_onto(floor_m);
    }

    /// Takes the pointer, or lets it go: with it, the pointer stays free to
    /// aim and its button is this plugin's. One plugin has it at a time, the
    /// last to take it.
    pub fn point(&mut self, on: bool) {
        if on {
            *self.lent.pointer = Some(self.plugin);
        } else if self.pointing() {
            *self.lent.pointer = None;
            self.lent.controller.pass_cells(false);
        }
    }

    /// While this plugin has the pointer, lets this client's own body fly
    /// through the cells, or has them hold it again: a builder flies through
    /// what they build. Letting the pointer go ends it.
    pub fn pass(&mut self, on: bool) {
        if self.pointing() {
            self.lent.controller.pass_cells(on);
        }
    }

    /// Whether this plugin has the pointer.
    pub fn pointing(&self) -> bool {
        *self.lent.pointer == Some(self.plugin)
    }

    /// The cells, to read: what a volume holds, what stands where.
    pub fn cells(&self) -> &Cells {
        self.lent.cells
    }

    /// A line of sight from a point of the world along a direction, unit,
    /// bent into the cells, and what it meets.
    pub fn sight(&self, from: DVec3, toward: DVec3) -> Sight {
        self.lent.cells.sight(self.lent.generator, from, toward)
    }

    /// Where a lattice point of a seat's cells is in the world.
    pub fn corner(&self, seat: Seat, p: [i32; 3]) -> DVec3 {
        self.lent.cells.corner(self.sphere(), seat, p)
    }

    /// Whether this session may change the cells: a builder's and an
    /// admin's, in a world that said so (DECISIONS 104). The server answers
    /// the same when a gesture is an op.
    pub fn may_change(&self) -> bool {
        self.lent.level.is_some_and(|level| level >= Level::Builder)
    }

    fn permitted(&self) -> Result<(), Refusal> {
        match self.may_change() {
            true => Ok(()),
            false => Err(Refusal::Level),
        }
    }

    /// The cells the volume of the plot a column is on holds, or would hold
    /// once opened: the room there is to build in. Why none can stand there,
    /// where none can.
    pub fn room(&self, point: SurfacePoint) -> Result<Span, Refusal> {
        self.lent.cells.room(self.lent.generator, point)
    }

    /// Opens the volume of the plot a column is on, where none stands.
    pub fn open(&mut self, point: SurfacePoint) -> Result<(), Refusal> {
        self.permitted()?;
        self.lent.cells.open(self.lent.generator, point)
    }

    /// Closes the volume over a column: what was built in it goes with it,
    /// and its plot is nature again. A change like any other, taken back as
    /// one. `Ok(false)` where no volume stands.
    pub fn close(&mut self, point: SurfacePoint) -> Result<bool, Refusal> {
        self.permitted()?;
        Ok(self.lent.cells.close(point))
    }

    /// Applies gestures to the cells of a seat as one change, taken back as
    /// one. `Ok(false)` when no cell changed.
    pub fn apply(&mut self, seat: impl Into<Seat>, gestures: &[Gesture]) -> Result<bool, Refusal> {
        self.permitted()?;
        Ok(self.lent.cells.apply(self.lent.generator, seat, gestures))
    }

    /// Shows the ghost of a gesture over a seat's cells, or of none: exactly
    /// the cells it would change.
    pub fn preview(&mut self, gesture: Option<(Seat, Gesture)>) {
        let sphere = self.sphere();
        self.lent.cells.preview(sphere, gesture);
    }

    /// Shows guides over the world, in place of those shown before, and
    /// none with none to show: each a box of a seat's cells as a faint grid.
    pub fn guide(&mut self, guides: &[Guide]) {
        let sphere = self.sphere();
        self.lent.cells.guide(sphere, guides);
    }

    /// Takes back the last change that landed.
    pub fn take_back(&mut self) -> Result<(), Refusal> {
        self.permitted()?;
        self.lent.cells.take_back(self.lent.generator);
        Ok(())
    }

    /// Puts back the last change taken back.
    pub fn put_back(&mut self) -> Result<(), Refusal> {
        self.permitted()?;
        self.lent.cells.put_back(self.lent.generator);
        Ok(())
    }
}

struct Plugged {
    plugin: Box<dyn Plugin>,
    /// Whether the world this client is in has it on, at this version.
    on: bool,
    /// The keys it asked for when it was plugged in.
    keys: Vec<KeyAsk>,
}

/// Every plugin a shell plugged in, in the order it did.
#[derive(Default)]
pub(crate) struct Plugins {
    held: Vec<Plugged>,
}

impl Plugins {
    pub(crate) fn plug(&mut self, plugin: Box<dyn Plugin>) {
        let keys = plugin.keys();
        self.held.push(Plugged {
            plugin,
            on: false,
            keys,
        });
    }

    /// Takes the world's statement: a plugin is on when the world says its
    /// name at the version held here, and one that was on and is no longer
    /// rests first. Returns what is on now, and a line for the log about
    /// each one the world speaks in another version.
    pub(crate) fn speak(
        &mut self,
        spoken: &[protocol::Plugin],
        mut lent: Lent<'_>,
    ) -> (Vec<PluginOn>, Vec<String>) {
        let mut apart = Vec::new();
        for held in &mut self.held {
            let said = spoken.iter().find(|said| said.name == held.plugin.name());
            let on = said.is_some_and(|said| said.version == held.plugin.version());
            if held.on && !on {
                rest(held, lent.again());
            }
            held.on = on;
            if let Some(said) = said.filter(|_| !on) {
                apart.push(format!(
                    "the world speaks {} {}, this client {}",
                    said.name,
                    said.version,
                    held.plugin.version()
                ));
            }
        }
        (self.on(), apart)
    }

    /// No world has spoken: every plugin is off.
    pub(crate) fn hush(&mut self, lent: Lent<'_>) -> Vec<PluginOn> {
        self.speak(&[], lent).0
    }

    /// Stands in for a world's word: every plugin plugged in is on, at the
    /// version it holds.
    pub(crate) fn rehearse(&mut self) -> Vec<PluginOn> {
        for held in &mut self.held {
            held.on = true;
        }
        self.on()
    }

    /// The world every plugin worked in is another now: each one that is on
    /// rests, and stays on.
    pub(crate) fn rest(&mut self, mut lent: Lent<'_>) {
        for held in self.held.iter_mut().filter(|held| held.on) {
            rest(held, lent.again());
        }
    }

    fn on(&self) -> Vec<PluginOn> {
        self.held
            .iter()
            .filter(|held| held.on)
            .map(|held| PluginOn {
                name: held.plugin.name().to_owned(),
                version: held.plugin.version(),
            })
            .collect()
    }

    /// Hands a command to the plugin it names. False when no plugin of that
    /// name is on.
    pub(crate) fn command(&mut self, name: &str, kind: &str, body: Value, lent: Lent<'_>) -> bool {
        let Some(held) = self.find(name) else {
            return false;
        };
        let mut host = lent.host(held.plugin.name());
        held.plugin.command(kind, body, &mut host);
        true
    }

    /// Hands a message from the server to the plugin its envelope names. One
    /// for a plugin that is off, or that nobody plugged in, is let pass.
    pub(crate) fn receive(&mut self, envelope: &protocol::Envelope, lent: Lent<'_>) {
        if let Some(held) = self.find(&envelope.plugin) {
            let mut host = lent.host(held.plugin.name());
            held.plugin
                .receive(&envelope.kind, &envelope.payload, &mut host);
        }
    }

    /// Whether a plugin that is on asks for a key with what is held with it:
    /// a shell keeps that key from whatever else would take it.
    pub(crate) fn asks(&self, code: &str, chord: Chord) -> bool {
        self.held
            .iter()
            .filter(|held| held.on)
            .any(|held| held.keys.iter().any(|ask| ask.is(code, chord)))
    }

    /// Hands each key that went down to the plugins that asked to hear it.
    pub(crate) fn keys(&mut self, pressed: &[(String, Chord)], mut lent: Lent<'_>) {
        for (code, chord) in pressed {
            for held in self.held.iter_mut().filter(|held| held.on) {
                let asked = held
                    .keys
                    .iter()
                    .find(|ask| !ask.held && ask.is(code, *chord));
                if let Some(name) = asked.map(|ask| ask.name) {
                    let mut host = lent.again().host(held.plugin.name());
                    held.plugin.key(name, &mut host);
                }
            }
        }
    }

    /// Gives every plugin that is on its turn in the frame.
    pub(crate) fn turn(&mut self, eye: &Eye, input: &Input, interrupted: bool, mut lent: Lent<'_>) {
        for held in self.held.iter_mut().filter(|held| held.on) {
            let name = held.plugin.name();
            let keys: Vec<&'static str> = held
                .keys
                .iter()
                .filter(|ask| ask.held && input.code_held(ask.code))
                .map(|ask| ask.name)
                .collect();
            let turn = Turn {
                eye: *eye,
                using: input.held(Key::Use) && *lent.pointer == Some(name),
                keys: &keys,
                interrupted,
            };
            held.plugin.turn(&turn, &mut lent.again().host(name));
        }
    }

    /// Whether a plugin that is on owes the world work.
    pub(crate) fn busy(&self) -> bool {
        self.held.iter().any(|held| held.on && held.plugin.busy())
    }

    /// Has every plugin that is on do at once the work it owes.
    pub(crate) fn settle(&mut self, mut lent: Lent<'_>) {
        for held in self.held.iter_mut().filter(|held| held.on) {
            let mut host = lent.again().host(held.plugin.name());
            held.plugin.settle(&mut host);
        }
    }

    fn find(&mut self, name: &str) -> Option<&mut Plugged> {
        self.held
            .iter_mut()
            .find(|held| held.on && held.plugin.name() == name)
    }
}

/// Has a plugin put down what it had in hand, and takes back from it what a
/// host lends one plugin at a time: the pointer, the ghost and the guides.
fn rest(held: &mut Plugged, lent: Lent<'_>) {
    let mut host = lent.host(held.plugin.name());
    held.plugin.rest(&mut host);
    if host.pointing() {
        host.point(false);
        host.preview(None);
        host.guide(&[]);
    }
}
