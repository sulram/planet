//! The items plugin's client half: the menu of things to place in the
//! world, the item in hand, the click that places it as a part, and the
//! gizmo that moves, turns and stretches a part picked (DECISIONS 111).
//! Building is the cells; this is everything placed that is no cell.
//!
//! An item is what the menu offers: a cube in a paint, a model from a
//! `.vox` file, named by its asset, and a picture, taken with the record of
//! its file. Each is a shape of part, and each enters here as a kind with the words it
//! needs. The parts are the core's: this plugin asks its host to place and
//! set them and holds no part.
//!
//! The seam, under `items.`: the commands `edit`, `take`, `pick`, `mode`,
//! `set`, `remove`, `paint`, `finish`, `edge` and `state`; the events
//! `hand`, `picked` and `refused`. `take` with a model carries its file:
//! `{"type":"items.take","item":"model","asset":"asset://<sha256>.vox"}`,
//! and the file is asked for at once, so its size is known when it is placed.

mod gizmo;
mod place;

use client::{Host, KeyAsk, Media, Part, PartId, Plugin, Shape, Turn};
use glam::{DQuat, EulerRot};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use voxel::{Edge, Finish, Paint};

use gizmo::Gizmo;
pub use gizmo::Mode;
pub use items_world::{NAME, VERSION};

/// What the menu offers to place.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Item {
    /// A cube of a metre, in the paint in hand.
    Cube,
    /// A model from a `.vox` file, the one taken in hand, a voxel an eighth
    /// of a cell (`client::VOXEL`).
    Model,
    /// A picture a metre tall in its own aspect, facing the eye, a pane
    /// nobody collides with: a screen in a light finish, a print in any
    /// other. Taken with its `media`, what the owner's client made of the
    /// file: its size in texels and three asset names, the original, a low
    /// version and a thumb.
    Picture,
}

/// Why an item was not handed over, or a part not placed or set. A front
/// end says it in its own words.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Refusal {
    /// Placing is a builder's and an admin's, and the world named this
    /// session a lower level.
    Level,
    /// The pose asked for is no pose.
    Pose,
    /// The part asked about is gone.
    Gone,
    /// The model in hand is still on its way: its size is not known yet.
    Unread,
}

impl From<client::PartRefusal> for Refusal {
    fn from(refusal: client::PartRefusal) -> Refusal {
        match refusal {
            client::PartRefusal::Level => Refusal::Level,
            client::PartRefusal::Pose => Refusal::Pose,
            client::PartRefusal::Gone => Refusal::Gone,
        }
    }
}

/// `items.edit`: open the menu, the pointer picking parts, or close it.
pub const EDIT: &str = "edit";
/// `items.take`: take an item in hand, or `null` to put it down. With
/// `asset`, the file of the model taken in hand, `asset://<sha256>.vox`. A
/// picture rides with its `media`, or with none while its file is still to
/// pick.
pub const TAKE: &str = "take";
/// `items.pick`: pick a part by its name, or `null` to let it go.
pub const PICK: &str = "pick";
/// `items.mode`: what a drag of a handle does.
pub const MODE: &str = "mode";
/// `items.set`: set the part picked: where it stands, how it is turned, in
/// degrees about its own axes, how long its sides are, in blocks, and
/// whether it is solid. What is left out stays.
pub const SET: &str = "set";
/// `items.remove`: remove the part picked.
pub const REMOVE: &str = "remove";
/// `items.paint`: pick a colour of the palette for what is placed next, and
/// for the part picked.
pub const PAINT: &str = "paint";
/// `items.finish`: how it takes the light.
pub const FINISH: &str = "finish";
/// `items.edge`: the line around each side.
pub const EDGE: &str = "edge";
/// `items.state`: say what is in hand, for a layer that just mounted.
pub const STATE: &str = "state";
/// `items.hand`: whether the menu is open, the item in hand, `null` when
/// none, the mode, the paint in hand, and the model's asset when one is in
/// hand, `null` otherwise.
pub const HAND: &str = "hand";
/// `items.picked`: the part picked, as it stands, or `null` when none.
pub const PICKED: &str = "picked";
/// `items.refused`: why what was asked was not done.
pub const REFUSED: &str = "refused";

#[derive(Deserialize)]
struct Edit {
    on: bool,
}

#[derive(Deserialize)]
struct Take {
    item: Option<Item>,
    #[serde(default)]
    asset: Option<String>,
    #[serde(default)]
    media: Option<MediaBody>,
}

/// A picture's record as the seam says it.
#[derive(Deserialize)]
struct MediaBody {
    width: u32,
    height: u32,
    thumb: String,
    low: String,
    full: String,
}

impl MediaBody {
    fn media(&self) -> Option<Media> {
        Media::parse(self.width, self.height, &self.thumb, &self.low, &self.full)
    }
}

#[derive(Deserialize)]
struct Pick {
    id: Option<u64>,
}

#[derive(Deserialize)]
struct Moded {
    mode: Mode,
}

#[derive(Deserialize, Default)]
struct Set {
    at: Option<[f64; 3]>,
    rotation: Option<[f64; 3]>,
    scale: Option<[f64; 3]>,
    solid: Option<bool>,
}

#[derive(Deserialize)]
struct Painted {
    paint: u8,
}

#[derive(Deserialize)]
struct Finished {
    finish: Finish,
}

#[derive(Deserialize)]
struct Edged {
    edge: Edge,
}

#[derive(Serialize)]
struct Hand {
    editing: bool,
    item: Option<Item>,
    mode: Mode,
    paint: u8,
    finish: Finish,
    edge: Edge,
    model: Option<String>,
}

/// A part as the panel shows it.
#[derive(Serialize)]
struct Picked {
    id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    at: Option<[f64; 3]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rotation: Option<[f64; 3]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scale: Option<[f64; 3]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    solid: Option<bool>,
}

#[derive(Serialize)]
struct Refused {
    reason: Refusal,
}

mod key {
    /// Open the menu, or close it.
    pub const ITEMS: &str = "items";
    /// Drop what is in hand: a drag, the part picked, the item, the menu.
    pub const CANCEL: &str = "cancel";
    /// What a drag of a handle does.
    pub const MOVE: &str = "move";
    pub const ROTATE: &str = "rotate";
    pub const SCALE: &str = "scale";
    /// Remove the part picked.
    pub const REMOVE: &str = "remove";
}

/// The colours a hand can pick: as many as the world's palette holds.
const PAINTS: u8 = client::PALETTE.len() as u8;

/// The menu, the item in hand, the paint it is placed in, and the part
/// picked.
pub struct Items {
    /// Whether the menu is open: the pointer is this plugin's, picking
    /// parts where no item is in hand.
    editing: bool,
    item: Option<Item>,
    mode: Mode,
    paint: u8,
    finish: Finish,
    edge: Edge,
    picked: Option<Gizmo>,
    /// The picture placed next, once its file is picked.
    picture: Option<Media>,
    /// The model in hand, by the hash of its file: what the model item
    /// places. None until a file is taken.
    model: Option<[u8; 32]>,
    /// Whether the pointer's button was down last turn.
    using: bool,
}

impl Default for Items {
    fn default() -> Self {
        Items {
            editing: false,
            item: None,
            mode: Mode::Move,
            paint: 0,
            finish: Finish::Matte,
            edge: Edge::None,
            picked: None,
            picture: None,
            model: None,
            using: false,
        }
    }
}

/// The plugin as a shell plugs it in.
pub fn plugin() -> Box<dyn Plugin> {
    Box::new(Items::default())
}

/// A turn about each of a part's own axes, degrees, from its rotation.
fn degrees(rotation: DQuat) -> [f64; 3] {
    let (x, y, z) = rotation.to_euler(EulerRot::XYZ);
    [x, y, z].map(f64::to_degrees)
}

fn radians(degrees: [f64; 3]) -> DQuat {
    let [x, y, z] = degrees.map(f64::to_radians);
    DQuat::from_euler(EulerRot::XYZ, x, y, z)
}

impl Items {
    /// The paint what is placed next is made of.
    fn made_of(&self) -> u8 {
        Paint {
            color: self.paint,
            finish: self.finish,
            edge: self.edge,
        }
        .byte()
    }

    /// Opens the menu or closes it. Open, the pointer is this plugin's.
    fn edit(&mut self, on: bool, host: &mut Host<'_>) {
        if on && !host.may_change() {
            return refuse(host, Refusal::Level);
        }
        self.editing = on;
        if !on {
            self.item = None;
            self.unpick(host);
        }
        host.point(on);
        self.say_hand(host);
    }

    /// Takes an item in hand or puts it down. The menu opens with it.
    pub(crate) fn take(&mut self, item: Option<Item>, host: &mut Host<'_>) {
        if item.is_some() && !host.may_change() {
            return refuse(host, Refusal::Level);
        }
        if item.is_some() {
            self.editing = true;
            self.unpick(host);
            host.point(true);
        }
        self.item = item;
        self.say_hand(host);
    }

    /// Takes an item in hand, with the file of a model when one is named:
    /// the file is asked for at once, so its size is known by the click.
    fn take_with(&mut self, item: Option<Item>, asset: Option<String>, host: &mut Host<'_>) {
        if let Some(name) = asset {
            let Some(Shape::Vox(hash)) = Shape::of_asset(&name) else {
                return host.reject(format!("`{name}` is no model's asset"));
            };
            if !host.may_change() {
                return refuse(host, Refusal::Level);
            }
            self.model = Some(hash);
            host.want_model(hash);
        }
        self.take(item, host);
    }

    /// Picks a part, or lets the one picked go.
    pub(crate) fn pick(&mut self, id: Option<PartId>, host: &mut Host<'_>) {
        self.picked = id.and_then(|id| Gizmo::new(host, id));
        if id.is_some() && self.picked.is_none() {
            refuse(host, Refusal::Gone);
        }
        self.say_picked(host);
    }

    fn unpick(&mut self, host: &mut Host<'_>) {
        if self.picked.take().is_some() {
            host.mark(&[]);
            self.say_picked(host);
        }
    }

    /// Sets the part picked as the panel asks.
    fn set(&mut self, set: Set, host: &mut Host<'_>) {
        let Some(gizmo) = &self.picked else {
            return refuse(host, Refusal::Gone);
        };
        let mut part = gizmo.part;
        if let Some(at) = set.at {
            part.pose.at = at;
        }
        if let Some(rotation) = set.rotation {
            part.pose.rotation = radians(rotation);
        }
        if let Some(scale) = set.scale {
            part.pose.scale = scale;
        }
        if let Some(solid) = set.solid {
            part.solid = solid;
        }
        let (id, seat) = (gizmo.id, gizmo.seat);
        self.apply(id, seat, part, host);
    }

    /// Sets a part and keeps the gizmo on it.
    fn apply(&mut self, id: PartId, seat: client::Seat, part: Part, host: &mut Host<'_>) {
        if let Err(refusal) = host.set_part(id, seat, part) {
            return refuse(host, refusal.into());
        }
        if let Some(gizmo) = &mut self.picked {
            gizmo.refresh(host);
        }
        self.say_picked(host);
    }

    /// Repaints the part picked, when one is, in the paint in hand.
    fn repaint(&mut self, host: &mut Host<'_>) {
        if let Some(gizmo) = &self.picked {
            let (id, seat, mut part) = (gizmo.id, gizmo.seat, gizmo.part);
            part.paint = self.made_of();
            self.apply(id, seat, part, host);
        }
    }

    fn remove(&mut self, host: &mut Host<'_>) {
        let Some(gizmo) = &self.picked else {
            return refuse(host, Refusal::Gone);
        };
        let id = gizmo.id;
        if let Err(refusal) = host.remove_part(id) {
            return refuse(host, refusal.into());
        }
        self.unpick(host);
    }

    fn say_hand(&self, host: &mut Host<'_>) {
        host.emit(
            HAND,
            &Hand {
                editing: self.editing,
                item: self.item,
                mode: self.mode,
                paint: self.paint,
                finish: self.finish,
                edge: self.edge,
                model: self.model.and_then(|hash| Shape::Vox(hash).asset()),
            },
        );
    }

    fn say_picked(&self, host: &mut Host<'_>) {
        let picked = match &self.picked {
            Some(gizmo) => Picked {
                id: Some(gizmo.id.0),
                at: Some(gizmo.part.pose.at),
                rotation: Some(degrees(gizmo.part.pose.rotation)),
                scale: Some(gizmo.part.pose.scale),
                solid: Some(gizmo.part.solid),
            },
            None => Picked {
                id: None,
                at: None,
                rotation: None,
                scale: None,
                solid: None,
            },
        };
        host.emit(PICKED, &picked);
    }

    /// One turn with the menu open and no item in hand: a handle dragged
    /// moves the part picked; a click picks a handle, a part, or nothing.
    fn edit_turn(&mut self, turn: &Turn<'_>, host: &mut Host<'_>) {
        let (from, toward) = turn.eye.sight();
        let pressed = turn.using && !self.using;
        let released = !turn.using && self.using;
        if let Some(gizmo) = &mut self.picked {
            if !gizmo.refresh(host) {
                self.unpick(host);
                return;
            }
        }
        // Shift held while stretching: every side alike.
        let uniform = turn.chord.shift;
        if let Some(gizmo) = &mut self.picked {
            if gizmo.drag.is_some() {
                if released || turn.interrupted {
                    gizmo.drag = None;
                } else if let Some(world) = gizmo.dragged(self.mode, uniform, from, toward)
                    && let Some((seat, pose)) = host.reseat(gizmo.seat.body(), &world)
                {
                    let (id, mut part) = (gizmo.id, gizmo.part);
                    part.pose = pose;
                    if let Err(refusal) = host.set_part(id, seat, part) {
                        refuse(host, refusal.into());
                    } else {
                        gizmo.refresh(host);
                        self.say_picked(host);
                    }
                }
            } else {
                let hit = gizmo.handle_hit(self.mode, from, toward);
                gizmo.hot = hit.map(|(axis, _)| axis);
                if pressed && let Some((axis, _)) = hit {
                    gizmo.press(self.mode, axis, from, toward);
                }
            }
        }
        let dragging = self
            .picked
            .as_ref()
            .is_some_and(|gizmo| gizmo.drag.is_some());
        let on_handle = self
            .picked
            .as_ref()
            .is_some_and(|gizmo| gizmo.hot.is_some());
        if pressed && !dragging && !on_handle {
            let id = host.pick(from, toward).map(|(id, _)| id);
            if id != self.picked.as_ref().map(|gizmo| gizmo.id) {
                self.pick(id, host);
            }
        }
        let marks = self.picked.as_ref().map(|gizmo| gizmo.marks(self.mode));
        host.mark(marks.as_deref().unwrap_or(&[]));
    }
}

fn refuse(host: &mut Host<'_>, reason: Refusal) {
    host.emit(REFUSED, &Refused { reason });
}

impl Plugin for Items {
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
            EDIT => {
                if let Some(Edit { on }) = read(body, host) {
                    self.edit(on, host);
                }
            }
            TAKE => {
                if let Some(Take { item, asset, media }) = read(body, host) {
                    if item == Some(Item::Picture) {
                        match media.as_ref().map(MediaBody::media) {
                            Some(None) => {
                                return host.reject("a picture names three asset:// files".into());
                            }
                            picked => self.picture = picked.flatten(),
                        }
                    }
                    self.take_with(item, asset, host);
                }
            }
            PICK => {
                if let Some(Pick { id }) = read(body, host) {
                    self.pick(id.map(PartId), host);
                }
            }
            MODE => {
                if let Some(Moded { mode }) = read(body, host) {
                    self.mode = mode;
                    self.say_hand(host);
                }
            }
            SET => {
                if let Some(set) = read(body, host) {
                    self.set(set, host);
                }
            }
            REMOVE => self.remove(host),
            PAINT => {
                if let Some(Painted { paint }) = read(body, host) {
                    self.paint = paint.min(PAINTS - 1);
                    self.say_hand(host);
                    self.repaint(host);
                }
            }
            FINISH => {
                if let Some(Finished { finish }) = read(body, host) {
                    self.finish = finish;
                    self.say_hand(host);
                    self.repaint(host);
                }
            }
            EDGE => {
                if let Some(Edged { edge }) = read(body, host) {
                    self.edge = edge;
                    self.say_hand(host);
                    self.repaint(host);
                }
            }
            STATE => {
                self.say_hand(host);
                self.say_picked(host);
            }
            _ => host.reject(format!("`{kind}` is no command of items")),
        }
    }

    fn keys(&self) -> Vec<KeyAsk> {
        vec![
            KeyAsk::pressed(key::ITEMS, "KeyI"),
            KeyAsk::pressed(key::CANCEL, "Escape"),
            KeyAsk::pressed(key::MOVE, "KeyG"),
            KeyAsk::pressed(key::ROTATE, "KeyR"),
            KeyAsk::pressed(key::SCALE, "KeyT"),
            KeyAsk::pressed(key::REMOVE, "Delete"),
            KeyAsk::pressed(key::REMOVE, "Backspace"),
        ]
    }

    fn key(&mut self, name: &str, host: &mut Host<'_>) {
        match name {
            key::ITEMS => self.edit(!self.editing, host),
            // What is in hand goes first: a drag, the part picked, the
            // item, then the menu itself.
            key::CANCEL if self.editing => {
                if let Some(gizmo) = &mut self.picked
                    && gizmo.drag.is_some()
                {
                    gizmo.drag = None;
                } else if self.picked.is_some() {
                    self.unpick(host);
                } else if self.item.is_some() {
                    self.take(None, host);
                } else {
                    self.edit(false, host);
                }
            }
            key::MOVE | key::ROTATE | key::SCALE if self.editing => {
                self.mode = match name {
                    key::MOVE => Mode::Move,
                    key::ROTATE => Mode::Rotate,
                    _ => Mode::Scale,
                };
                self.say_hand(host);
            }
            key::REMOVE if self.picked.is_some() => self.remove(host),
            _ => {}
        }
    }

    /// One turn: a tap of the secondary button on a part picks it, from
    /// anywhere, and opens the menu on it; with an item in hand a click
    /// places it; with the menu open the gizmo works.
    fn turn(&mut self, turn: &Turn<'_>, host: &mut Host<'_>) {
        if turn.inspected && host.may_change() {
            let (from, toward) = turn.eye.sight();
            if let Some((id, _)) = host.pick(from, toward) {
                if !self.editing {
                    self.editing = true;
                    host.point(true);
                }
                self.item = None;
                self.say_hand(host);
                self.pick(Some(id), host);
            }
        }
        if !self.editing {
            self.using = turn.using;
            return;
        }
        match self.item {
            Some(item) => {
                let pressed = turn.using && !self.using && !turn.interrupted;
                if pressed {
                    self.place(item, &turn.eye, host);
                }
            }
            None => self.edit_turn(turn, host),
        }
        self.using = turn.using;
    }

    fn rest(&mut self, host: &mut Host<'_>) {
        self.using = false;
        if self.editing {
            self.edit(false, host);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use client::{Client, Event, Input, Key, Level, Recipe};

    fn client() -> Client {
        let mut client = Client::new(Recipe::new(1)).unwrap();
        client.plug(plugin());
        client.rehearse(Level::Admin);
        client.set_view([1600, 900]);
        client
    }

    fn events(client: &mut Client) -> Vec<Value> {
        client
            .drain_events()
            .into_iter()
            .filter_map(|event| match event {
                Event::Plugin(event) => Some(event),
                _ => None,
            })
            .collect()
    }

    fn click(client: &mut Client, at: [f32; 2]) {
        let mut input = Input::default();
        input.pointer = Some(at);
        input.key(Key::Use, true);
        client.update(1.0 / 60.0, &mut input);
        input.key(Key::Use, false);
        client.update(1.0 / 60.0, &mut input);
    }

    #[test]
    fn a_click_with_a_cube_in_hand_places_a_part_and_picks_it() {
        let mut client = client();
        client.command_json(r#"{"type":"items.paint","paint":11}"#);
        client.command_json(r#"{"type":"items.take","item":"cube"}"#);
        let hand = events(&mut client);
        assert_eq!(hand.last().unwrap()["item"], "cube");
        assert!(client.pointing());
        click(&mut client, [0.5, 0.7]);
        let placed: Vec<(PartId, Part)> = client
            .parts()
            .all()
            .map(|(id, _, part)| (id, *part))
            .collect();
        assert_eq!(placed.len(), 1);
        assert_eq!(placed[0].1.paint, 11);
        assert!(placed[0].1.solid);
        // Placed, the part is picked and the hand is empty: the menu stays
        // open to move it.
        let said = events(&mut client);
        let picked = said
            .iter()
            .rev()
            .find(|e| e["type"] == "items.picked")
            .unwrap();
        assert_eq!(picked["id"], placed[0].0.0);
        let hand = said
            .iter()
            .rev()
            .find(|e| e["type"] == "items.hand")
            .unwrap();
        assert!(hand["item"].is_null() && hand["editing"] == true);
        assert!(client.pointing());
        // Closed, the pointer goes back.
        client.command_json(r#"{"type":"items.edit","on":false}"#);
        assert!(!client.pointing());
    }

    #[test]
    fn the_panel_sets_the_part_picked_and_a_key_removes_it() {
        let mut client = client();
        client.command_json(r#"{"type":"items.take","item":"cube"}"#);
        click(&mut client, [0.5, 0.7]);
        events(&mut client);
        client.command_json(
            r#"{"type":"items.set","rotation":[0,0,90],"scale":[4,2,1],"solid":false}"#,
        );
        let said = events(&mut client);
        let picked = said
            .iter()
            .rev()
            .find(|e| e["type"] == "items.picked")
            .unwrap();
        assert_eq!(picked["scale"], serde_json::json!([4.0, 2.0, 1.0]));
        assert!((picked["rotation"][2].as_f64().unwrap() - 90.0).abs() < 1e-6);
        assert_eq!(picked["solid"], false);
        let part = client.parts().all().next().unwrap().2;
        assert!(!part.solid);
        // A pose that is none is refused and the part keeps its own.
        client.command_json(r#"{"type":"items.set","scale":[0,2,1]}"#);
        let said = events(&mut client);
        assert_eq!(said.last().unwrap()["reason"], "pose");
        client.command_json(r#"{"type":"items.remove"}"#);
        assert_eq!(client.parts().all().count(), 0);
        let said = events(&mut client);
        assert!(said.last().unwrap()["id"].is_null());
    }

    /// A `.vox` of one voxel, as MagicaVoxel writes one: `SIZE` and `XYZI`
    /// under `MAIN`, the default palette.
    fn one_voxel() -> Vec<u8> {
        let chunk = |id: &[u8; 4], content: &[u8]| {
            let mut out = id.to_vec();
            out.extend((content.len() as u32).to_le_bytes());
            out.extend(0u32.to_le_bytes());
            out.extend(content);
            out
        };
        let mut size = Vec::new();
        for n in [1u32, 1, 1] {
            size.extend(n.to_le_bytes());
        }
        let mut xyzi = 1u32.to_le_bytes().to_vec();
        xyzi.extend([0, 0, 0, 1]);
        let children = [chunk(b"SIZE", &size), chunk(b"XYZI", &xyzi)].concat();
        let mut main = b"MAIN".to_vec();
        main.extend(0u32.to_le_bytes());
        main.extend((children.len() as u32).to_le_bytes());
        main.extend(children);
        let mut file = b"VOX ".to_vec();
        file.extend(150u32.to_le_bytes());
        file.extend(main);
        file
    }

    #[test]
    fn a_model_taken_in_hand_is_fetched_and_placed_by_a_click() {
        let mut client = client();
        let asset = format!("asset://{}.vox", "ab".repeat(32));
        client.command_json(&format!(
            r#"{{"type":"items.take","item":"model","asset":"{asset}"}}"#
        ));
        let hand = events(&mut client);
        let hand = hand.last().unwrap();
        assert_eq!(hand["item"], "model");
        assert_eq!(hand["model"], asset);
        // The file is asked for at once, by its asset name.
        let asked = client.drain_asset_requests();
        let asked = asked
            .iter()
            .find(|request| request.path == asset)
            .expect("the model is asked for");
        // A click before it arrives places nothing: its size is unknown.
        click(&mut client, [0.5, 0.7]);
        assert_eq!(events(&mut client).last().unwrap()["reason"], "unread");
        assert_eq!(client.parts().all().count(), 0);
        // The file arrives: the next click places it, a voxel an eighth of
        // a cell, its mesh is announced, and the part is picked.
        client.asset_loaded(asked.id, Ok(one_voxel()));
        assert!(!client.drain_model_changes().is_empty());
        click(&mut client, [0.5, 0.7]);
        let said = events(&mut client);
        assert!(said.iter().all(|e| e["type"] != "items.refused"));
        let placed: Vec<Part> = client.parts().all().map(|(_, _, part)| *part).collect();
        assert_eq!(placed.len(), 1);
        assert!(matches!(placed[0].shape, Shape::Vox(_)));
        assert_eq!(placed[0].pose.scale, [0.125, 0.125, 0.125]);
        assert!(
            said.iter()
                .any(|e| e["type"] == "items.picked" && !e["id"].is_null())
        );
        // A name that is no model's is rejected and leaves the hand as it is.
        client.command_json(r#"{"type":"items.take","item":"model","asset":"asset://nope.png"}"#);
        assert!(
            client
                .drain_events()
                .iter()
                .any(|event| matches!(event, Event::Rejected { .. }))
        );
        // A model with none picked places nothing and refuses nothing.
        let mut fresh = self::client();
        fresh.command_json(r#"{"type":"items.take","item":"model"}"#);
        assert_eq!(events(&mut fresh).last().unwrap()["model"], Value::Null);
        click(&mut fresh, [0.5, 0.7]);
        assert!(events(&mut fresh).is_empty());
        assert_eq!(fresh.parts().all().count(), 0);
    }

    #[test]
    fn a_picture_in_hand_is_placed_facing_the_eye_and_asks_for_its_level() {
        let mut client = client();
        let hex = |byte: &str| byte.repeat(32);
        let media = format!(
            r#"{{"width":3000,"height":2000,"thumb":"asset://{}.webp","low":"asset://{}.webp","full":"asset://{}.jpg"}}"#,
            hex("01"),
            hex("02"),
            hex("03")
        );
        // Taken with no file yet, a click places nothing.
        client.command_json(r#"{"type":"items.take","item":"picture"}"#);
        click(&mut client, [0.5, 0.7]);
        assert_eq!(client.parts().all().count(), 0);
        client.command_json(&format!(
            r#"{{"type":"items.take","item":"picture","media":{media}}}"#
        ));
        assert!(
            !client
                .drain_events()
                .iter()
                .any(|event| matches!(event, Event::Rejected { .. })),
            "a sound picture is taken quietly"
        );
        click(&mut client, [0.5, 0.7]);

        let frame = client.settled_frame();
        assert_eq!(frame.pictures.len(), 1);
        assert!(
            frame
                .models
                .iter()
                .all(|model| model.transform != frame.pictures[0].transform),
            "a picture is no model"
        );
        let picture = frame.pictures[0];
        assert_eq!(picture.picture, None, "a card until its level lands");
        // A metre tall, in its aspect, its face turned to the eye.
        let (width, height) = (
            picture.transform.matrix3.x_axis.length(),
            picture.transform.matrix3.y_axis.length(),
        );
        assert!((height - 1.0).abs() < 0.05, "{height}");
        assert!((width / height - 1.5).abs() < 0.05, "{width} by {height}");
        let to_eye = frame.camera.position - picture.transform.translation;
        assert!(
            picture.transform.matrix3.z_axis.dot(to_eye) > 0.0,
            "faces the eye"
        );

        // Close and in view, the low version is what is asked for, as pixels.
        let asked = client.drain_asset_requests();
        let levels: Vec<_> = asked
            .iter()
            .filter(|request| matches!(request.form, client::Form::Picture { .. }))
            .collect();
        assert_eq!(levels.len(), 1, "{asked:?}");
        assert_eq!(levels[0].path, format!("asset://{}.webp", hex("02")));
        client.picture_loaded(
            levels[0].id,
            Ok(scene::Image {
                width: 1024,
                height: 683,
                rgba: vec![128; 1024 * 683 * 4],
            }),
        );
        let frame = client.settled_frame();
        assert!(
            frame.pictures[0].picture.is_some(),
            "the level landed is shown"
        );
        assert_eq!(client.drain_picture_changes().len(), 1);

        // No picture names three files: refused, and nothing changes hands.
        client.command_json(
            r#"{"type":"items.take","item":"picture","media":{"width":1,"height":1,"thumb":"a","low":"b","full":"c"}}"#,
        );
        assert!(
            client
                .drain_events()
                .iter()
                .any(|event| matches!(event, Event::Rejected { .. }))
        );
        // The picture placed is picked: a cube beside it is placed off it.
        client.command_json(r#"{"type":"items.take","item":"cube"}"#);
        click(&mut client, [0.3, 0.7]);
        assert_eq!(client.parts().all().count(), 2, "a cube beside the picture");
    }

    #[test]
    fn a_tap_of_the_secondary_button_picks_a_part_from_anywhere() {
        let mut client = client();
        client.command_json(r#"{"type":"items.take","item":"cube"}"#);
        click(&mut client, [0.5, 0.7]);
        client.command_json(r#"{"type":"items.edit","on":false}"#);
        events(&mut client);
        assert!(!client.pointing());
        let mut input = Input::default();
        input.pointer = Some([0.5, 0.7]);
        input.inspect();
        client.update(1.0 / 60.0, &mut input);
        let said = events(&mut client);
        let picked = said
            .iter()
            .rev()
            .find(|e| e["type"] == "items.picked")
            .unwrap();
        assert!(picked["id"].is_number());
        let hand = said
            .iter()
            .rev()
            .find(|e| e["type"] == "items.hand")
            .unwrap();
        assert_eq!(hand["editing"], true);
        assert!(client.pointing());
    }

    #[test]
    fn a_visitor_opens_nothing() {
        let mut client = Client::new(Recipe::new(1)).unwrap();
        client.plug(plugin());
        client.rehearse(Level::SignedIn);
        client.command_json(r#"{"type":"items.edit","on":true}"#);
        let said = events(&mut client);
        assert_eq!(said.last().unwrap()["reason"], "level");
        assert!(!client.pointing());
    }
}
