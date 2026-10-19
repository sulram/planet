//! The host of plugins on the client: who is plugged in, which of them the
//! world has on, and what each is offered (DECISIONS 88, 93, 98, 108).
//!
//! A plugin's client half is a crate of its own that imports this one. The
//! core names no plugin: a shell plugs in what its version carries, the
//! world's statement says which are on, and a command, a frame from the
//! server, a key or a turn reaches a plugin by the name it gave.
//!
//! A plugin speaks to the core's systems through its [`Host`]: the ground,
//! the cells, the parts, the body. What a system holds is read freely. What
//! changes it is an op, and the host asks whether this session may before
//! the system sees it.

use glam::{DAffine3, DQuat, DVec3};
use part::{Part, PartId, Pose as PartPose};
use protocol::Body;
use serde::Serialize;
use serde_json::Value;
use topology::{QuadSphere, SurfacePoint};
use voxel::{Gesture, Span};
use worldgen::Generator;

use crate::cells::{self, Cells, Guide, Refusal, Seat, Sight};
use crate::collision;
use crate::controller::Controller;
use crate::history::{History, Owner};
use crate::input::{Chord, Input, Key, KeyAsk};
use crate::parts::{Mark, PartRefusal, Parts};
use crate::place::Pose;
use crate::seam::{Event, Level, PluginOn};
use crate::session::Session;

/// How far from the moon's centre a point of the world is the moon's,
/// metres: its radius and the band over it where anything stands.
const MOON_REACH_M: f64 = worldgen::MOON_RADIUS_M + 2000.0;

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
    /// What is held right now, command and shift. The core's keys, so a
    /// plugin reads them here and never asks for them: a shift asked for
    /// would take the sprint from the body.
    pub chord: Chord,
    /// The keys this plugin asked to read held that are down, by name.
    pub keys: &'a [&'static str],
    /// Whether the shell let everything go since the last turn: what was
    /// half done is dropped, never landed.
    pub interrupted: bool,
    /// Whether the pointer's secondary button was tapped since the last
    /// turn, where the eye's pointer is: heard by every plugin, whoever
    /// has the pointer, so a thing in the world is picked to be looked at
    /// again from anywhere.
    pub inspected: bool,
}

impl Turn<'_> {
    /// Whether a key this plugin asked to read held is down.
    pub fn held(&self, name: &str) -> bool {
        self.keys.contains(&name)
    }
}

/// This client's own body in the world, as a hand that places something in
/// front of it reads it.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Standing {
    /// From the planet's centre, metres.
    pub position: DVec3,
    pub up: DVec3,
    pub facing: DVec3,
}

/// Where the feet of this client's own body are.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Feet {
    /// The seat they are over: a sector of the body that holds them.
    pub seat: Seat,
    /// The column of it they are over, in that body's own blocks.
    pub point: SurfacePoint,
    /// How high over the datum of that body, metres.
    pub height_m: f64,
}

/// What of the core a host lends a plugin for one call.
pub(crate) struct Lent<'a> {
    pub session: &'a mut Session,
    pub events: &'a mut Vec<Event>,
    pub generator: &'a Generator,
    pub cells: &'a mut Cells,
    pub parts: &'a mut Parts,
    pub history: &'a mut History,
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
            parts: self.parts,
            history: self.history,
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

    /// How high the ground of a seat stands at a column of it, in blocks:
    /// the ground in full detail, as a body stands on it.
    pub fn ground(&self, seat: Seat, point: SurfacePoint) -> f64 {
        cells::ground(self.lent.generator, seat, point)
    }

    /// This client's own body in the world: where it is, from the planet's
    /// centre, which way is up for it, and which way it faces, both unit.
    pub fn body(&self) -> Standing {
        let controller = &self.lent.controller;
        Standing {
            position: controller.position(),
            up: controller.up(),
            facing: controller.facing(),
        }
    }

    /// Where the feet of this client's own body are.
    pub fn feet(&self) -> Feet {
        let controller = &self.lent.controller;
        let (seat, point) = controller.over(self.lent.generator);
        Feet {
            seat,
            point,
            height_m: controller.height_m(),
        }
    }

    /// Lifts the body onto a floor `floor_m` over the datum, when it stands
    /// lower.
    pub fn lift(&mut self, floor_m: f64) {
        self.lent.controller.lift_onto(floor_m);
    }

    /// Where a point of the world is, as feet standing there: the seat under
    /// it, the column of it there, and its height over that body's datum.
    /// On the moon when it is within the moon's reach, else on the planet.
    pub fn feet_at(&self, point: DVec3) -> Feet {
        let generator = self.lent.generator;
        let moon = self.lent.cells.centre(Body::Moon);
        let body = match (point - moon).length() < MOON_REACH_M {
            true => Body::Moon,
            false => Body::Planet,
        };
        let sphere = seat::sphere_of(generator, body);
        let from_centre = point - self.lent.cells.centre(body);
        let point = sphere.blocks().surface_point(from_centre.to_array());
        Feet {
            seat: Seat::on(body, point.sector),
            point,
            height_m: from_centre.length() - sphere.radius_m(),
        }
    }

    /// Where a line of sight from a point of the world along a direction,
    /// unit, goes into the ground of a body within `reach_m`, as feet
    /// standing there, and how far along, metres. `None` when it meets no
    /// ground that far.
    pub fn ground_hit(&self, from: DVec3, toward: DVec3, reach_m: f64) -> Option<(Feet, f64)> {
        let generator = self.lent.generator;
        let bodies = [Body::Planet, Body::Moon].into_iter().filter(|&body| {
            // A line nowhere near a body meets no ground of it.
            let centre = self.lent.cells.centre(body);
            let radius_m = seat::sphere_of(generator, body).radius_m();
            let to = centre - from;
            let along = to.dot(toward).clamp(0.0, reach_m);
            (to - toward * along).length() < radius_m + MOON_REACH_M
        });
        let hits = bodies.filter_map(|body| {
            let sphere = seat::sphere_of(generator, body);
            let from = from - self.lent.cells.centre(body);
            let under = |at: DVec3| {
                let direction = at.normalize_or(DVec3::Y).to_array();
                at.length() - sphere.radius_m() < seat::ground_m(generator, body, direction)
            };
            let hit_m = collision::ground_run_m(from, toward, reach_m, under)?;
            let at = from + toward * hit_m;
            let point = sphere.blocks().surface_point(at.to_array());
            let feet = Feet {
                seat: Seat::on(body, point.sector),
                point,
                height_m: at.length() - sphere.radius_m(),
            };
            Some((hit_m, feet))
        });
        hits.min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(away_m, feet)| (feet, away_m))
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
    /// bent into the cells, and what it meets first: a cell, the ground of a
    /// volume, or a side of `room`, the box a hand builds in, where the line
    /// leaves it.
    pub fn sight(&self, from: DVec3, toward: DVec3, room: Option<(Seat, Span)>) -> Sight {
        self.lent
            .cells
            .sight(self.lent.generator, from, toward, room)
    }

    /// Where a lattice point of a seat's cells is in the world.
    pub fn corner(&self, seat: Seat, p: [i32; 3]) -> DVec3 {
        self.lent.cells.corner(self.lent.generator, seat, p)
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

    /// The cells the volume of the plot a column of a seat is on holds, or
    /// would hold once opened: the room there is to build in. Why none can
    /// stand there, where none can.
    pub fn room(&self, seat: Seat, point: SurfacePoint) -> Result<Span, Refusal> {
        self.lent.cells.room(self.lent.generator, seat, point)
    }

    /// Opens the volume of the plot a column of a seat is on, where none
    /// stands: room to build in, with nothing built. A stroke starts on its
    /// ground.
    pub fn open(&mut self, seat: Seat, point: SurfacePoint) -> Result<(), Refusal> {
        self.permitted()?;
        self.lent.cells.open(self.lent.generator, seat, point)
    }

    /// A change of the cells landed here: it enters the one history.
    fn did_cells(&mut self) {
        let ask = self.lent.cells.last_ask();
        self.lent.history.did(Owner::Cells, ask);
    }

    /// Closes the volume over a column of a seat: what was built in it goes
    /// with it, and its plot is nature again. A change like any other, taken
    /// back as one. `Ok(false)` where no volume stands.
    pub fn close(&mut self, seat: Seat, point: SurfacePoint) -> Result<bool, Refusal> {
        self.permitted()?;
        let closed = self.lent.cells.close(seat, point);
        if closed {
            self.did_cells();
        }
        Ok(closed)
    }

    /// Applies gestures to the cells of a seat as one change, taken back as
    /// one. `Ok(false)` when no cell changed.
    pub fn apply(&mut self, seat: impl Into<Seat>, gestures: &[Gesture]) -> Result<bool, Refusal> {
        self.permitted()?;
        let changed = self.lent.cells.apply(self.lent.generator, seat, gestures);
        if changed {
            self.did_cells();
        }
        Ok(changed)
    }

    /// Shows the ghost of a gesture over a seat's cells, or of none: exactly
    /// the cells it would change.
    pub fn preview(&mut self, gesture: Option<(Seat, Gesture)>) {
        self.lent.cells.preview(gesture);
    }

    /// Shows guides over the world, in place of those shown before, and
    /// none with none to show: each a box of a seat's cells as a faint grid.
    pub fn guide(&mut self, guides: &[Guide]) {
        self.lent.cells.guide(self.lent.generator, guides);
    }

    /// The parts, to read: what stands where.
    pub fn parts(&self) -> &Parts {
        self.lent.parts
    }

    /// The nearest part a line from a point of the world along a direction,
    /// unit, meets, and how far along, metres.
    pub fn pick(&self, from: DVec3, toward: DVec3) -> Option<(PartId, f64)> {
        self.lent.parts.pick(self.lent.generator, from, toward)
    }

    /// Whether this session may change the parts, in the parts' words.
    fn may_place(&self) -> Result<(), PartRefusal> {
        match self.may_change() {
            true => Ok(()),
            false => Err(PartRefusal::Level),
        }
    }

    /// The world from a part's own space, from the planet's centre: where
    /// it stands this frame. `None` for a name nothing holds.
    pub fn part_transform(&self, id: PartId) -> Option<DAffine3> {
        self.lent.parts.transform(self.lent.generator, id)
    }

    /// The seat and pose a part standing in the world would have, on a
    /// body: how a part is moved. A gizmo changes the world transform and
    /// asks here what pose that is. `None` for a transform that is no pose.
    pub fn reseat(&self, body: Body, world: &DAffine3) -> Option<(Seat, PartPose)> {
        self.lent.parts.reseat(self.lent.generator, body, world)
    }

    /// Shows marks over the world, in place of those shown before, and none
    /// with none to show: cubes in a paint drawn over everything, for a hand
    /// to see.
    pub fn mark(&mut self, marks: &[Mark]) {
        self.lent.parts.mark(marks);
    }

    /// Whether a pose is one a part on a seat can have: sound, and inside
    /// the seat's sector.
    fn sound(&self, seat: Seat, part: &Part) -> Result<(), PartRefusal> {
        let pose = &part.pose;
        match part.is_sound() && seat::inside(seat.sphere(self.lent.generator), pose) {
            true => Ok(()),
            false => Err(PartRefusal::Pose),
        }
    }

    /// Places a part on a seat, where its pose says: a change like any
    /// other. The name it is given.
    pub fn place_part(&mut self, seat: Seat, part: Part) -> Result<PartId, PartRefusal> {
        self.may_place()?;
        self.sound(seat, &part)?;
        let id = self.lent.parts.place(self.lent.generator, seat, part);
        let ask = self.lent.parts.last_ask();
        self.lent.history.did(Owner::Parts, ask);
        Ok(id)
    }

    /// Sets a part by its name: moved, turned, stretched, repainted, made
    /// solid or not, or seated elsewhere.
    pub fn set_part(&mut self, id: PartId, seat: Seat, part: Part) -> Result<(), PartRefusal> {
        self.may_place()?;
        self.sound(seat, &part)?;
        match self.lent.parts.set(self.lent.generator, id, seat, part) {
            true => {
                // A set goes to the world at a cadence, under no number yet.
                self.lent.history.set(id.0, None);
                Ok(())
            }
            false => Err(PartRefusal::Gone),
        }
    }

    /// Asks for a model's file, once, so a hand knows its size before it
    /// places it. What arrives is kept and drawn for every part that names
    /// it.
    pub fn want_model(&mut self, hash: [u8; 32]) {
        self.lent.parts.want(hash);
    }

    /// How many voxels a model has along each axis, once its file arrived.
    pub fn model_size(&self, hash: [u8; 32]) -> Option<[u32; 3]> {
        self.lent.parts.size_of(hash)
    }

    /// Removes a part by its name.
    pub fn remove_part(&mut self, id: PartId) -> Result<(), PartRefusal> {
        self.may_place()?;
        match self.lent.parts.remove(id) {
            true => {
                let ask = self.lent.parts.last_ask();
                self.lent.history.did(Owner::Parts, ask);
                Ok(())
            }
            false => Err(PartRefusal::Gone),
        }
    }

    /// Whether there is a change to take back, and one to put back, of
    /// whatever owner (DECISIONS 115).
    pub fn history(&self) -> (bool, bool) {
        self.lent.history.state()
    }

    /// Takes back the last change that landed, whatever owner it was of.
    pub fn take_back(&mut self) -> Result<(), Refusal> {
        self.permitted()?;
        match self.lent.history.take_back() {
            Some(Owner::Cells) => self.lent.cells.take_back(self.lent.generator),
            Some(Owner::Parts) => self.lent.parts.restore(self.lent.generator, true),
            None => {}
        }
        Ok(())
    }

    /// Puts back the last change taken back.
    pub fn put_back(&mut self) -> Result<(), Refusal> {
        self.permitted()?;
        match self.lent.history.put_back() {
            Some(Owner::Cells) => self.lent.cells.put_back(self.lent.generator),
            Some(Owner::Parts) => self.lent.parts.restore(self.lent.generator, false),
            None => {}
        }
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
    pub(crate) fn command(
        &mut self,
        name: &str,
        kind: &str,
        body: Value,
        mut lent: Lent<'_>,
    ) -> bool {
        let Some(held) = self.find(name) else {
            return false;
        };
        let before = *lent.pointer;
        let mut host = lent.again().host(held.plugin.name());
        held.plugin.command(kind, body, &mut host);
        self.handed(before, lent);
        true
    }

    /// Hands a message from the server to the plugin its envelope names. One
    /// for a plugin that is off, or that nobody plugged in, is let pass.
    pub(crate) fn receive(&mut self, envelope: &protocol::Envelope, mut lent: Lent<'_>) {
        if let Some(held) = self.find(&envelope.plugin) {
            let before = *lent.pointer;
            let mut host = lent.again().host(held.plugin.name());
            held.plugin
                .receive(&envelope.kind, &envelope.payload, &mut host);
            self.handed(before, lent);
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
        let before = *lent.pointer;
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
        self.handed(before, lent);
    }

    /// Gives every plugin that is on its turn in the frame.
    pub(crate) fn turn(
        &mut self,
        eye: &Eye,
        input: &Input,
        (interrupted, inspected): (bool, bool),
        mut lent: Lent<'_>,
    ) {
        let before = *lent.pointer;
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
                chord: input.chord,
                keys: &keys,
                interrupted,
                inspected,
            };
            held.plugin.turn(&turn, &mut lent.again().host(name));
        }
        self.handed(before, lent);
    }

    /// Whether a plugin that is on owes the world work.
    pub(crate) fn busy(&self) -> bool {
        self.held.iter().any(|held| held.on && held.plugin.busy())
    }

    /// Has every plugin that is on do at once the work it owes.
    pub(crate) fn settle(&mut self, mut lent: Lent<'_>) {
        let before = *lent.pointer;
        for held in self.held.iter_mut().filter(|held| held.on) {
            let mut host = lent.again().host(held.plugin.name());
            held.plugin.settle(&mut host);
        }
        self.handed(before, lent);
    }

    /// Has the plugin with the pointer put it down, with what it had in
    /// hand: what a front end asks when a panel of its own opens.
    pub(crate) fn put_down(&mut self, mut lent: Lent<'_>) {
        if let Some(name) = *lent.pointer
            && let Some(held) = self.find(name)
        {
            rest(held, lent.again());
        }
    }

    /// After the plugins acted: one plugin has the pointer at a time, so the
    /// one it was taken from rests, and the seam hears whose it is now.
    fn handed(&mut self, before: Option<&'static str>, mut lent: Lent<'_>) {
        let now = *lent.pointer;
        if before == now {
            return;
        }
        if let (Some(former), Some(_)) = (before, now)
            && let Some(held) = self.find(former)
        {
            let mut host = lent.again().host(held.plugin.name());
            held.plugin.rest(&mut host);
            host.preview(None);
            host.guide(&[]);
        }
        lent.events.push(Event::PointerChanged {
            plugin: now.map(str::to_string),
        });
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
    let had_pointer = host.pointing();
    held.plugin.rest(&mut host);
    if host.pointing() {
        host.point(false);
        host.preview(None);
        host.guide(&[]);
    }
    if had_pointer {
        host.lent
            .events
            .push(Event::PointerChanged { plugin: None });
    }
}
