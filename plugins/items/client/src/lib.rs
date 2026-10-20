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

mod cross;
mod gizmo;
mod place;

use client::{
    Clip, Host, ImageKind, KeyAsk, Landmarked, Media, Name, Part, PartId, Plugin, Portal, Shape,
    Turn,
};

use glam::{DQuat, DVec3, EulerRot};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use topology::BLOCK_M;
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
    /// A video: a picture whose file is an MP4, placed as a picture is and
    /// heard within its reach. Taken with its `media`, as a picture is.
    Video,
    /// A sound: a file played where it is placed, looping, heard within its
    /// reach; nothing is drawn of it but its gizmo when picked, so the list
    /// of what is near finds it. Taken with its `clip`: the asset name of
    /// the file and how long it plays.
    Sound,
    /// A landmark: a star a metre across lying where it is placed, bearing
    /// the name in hand, in the colour in hand and always lit with its own
    /// light, a lamp; nobody collides with it. A place one travels to: the
    /// chart lists every landmark of the world with its place code.
    Landmark,
    /// A portal: a standing ring two metres across, its rim in the colour in
    /// hand and always lit with its own light, a lamp, round a dark pane,
    /// facing the body as a picture does; nobody collides with it. It leads to the landmark in hand, by
    /// the name the world gave it: a body that walks through its pane is
    /// taken to stand on that star, facing the way it points.
    Portal,
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
    /// The name given is no name: empty, over sixty four bytes, or with a
    /// control character in it; or the part picked bears no name.
    Name,
    /// The destination given is no landmark this client holds, or the part
    /// picked is no portal.
    Nowhere,
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
/// With `media`, another picture takes the place of the one picked, as
/// wide as its aspect makes it at the same height.
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
/// `items.list`: say the parts near the body, nearest first, with `near`.
pub const LIST: &str = "list";
/// `items.chart`: say the landmarks of the world, wherever they stand, with
/// `landmarks`, once the world has answered. At any level.
pub const CHART: &str = "chart";
/// `items.hand`: whether the menu is open, the item in hand, `null` when
/// none, the mode, the paint in hand, and the model's asset when one is in
/// hand, `null` otherwise.
pub const HAND: &str = "hand";
/// `items.picked`: the part picked, as it stands, or `null` when none.
pub const PICKED: &str = "picked";
/// `items.refused`: why what was asked was not done.
pub const REFUSED: &str = "refused";
/// `items.near`: the parts within a plot's width of the body, the nearest
/// first: each by its name, what kind of item it is, how far, its paint and
/// whether it is solid.
pub const NEAR: &str = "near";
/// `items.landmarks`: the landmarks of the world, the nearest first: each
/// by its name in the world, what it is called, its paint, how far it is
/// and the place code that takes a body to it.
pub const LANDMARKS: &str = "landmarks";
/// `items.crossed`: the body walked through a portal: which, the landmark
/// it leads to, and the place the body now stands, `null` when the portal
/// leads nowhere and the body stayed.
pub const CROSSED: &str = "crossed";

#[derive(Serialize)]
struct Crossed {
    portal: u64,
    to: Option<u64>,
    place: Option<String>,
}

#[derive(Serialize)]
struct LandmarkOne {
    id: u64,
    name: String,
    paint: u8,
    /// Infinite on another body.
    away_m: f64,
    place: String,
}

#[derive(Serialize)]
struct Landmarks {
    landmarks: Vec<LandmarkOne>,
}

/// How far from the body a part is listed, metres: a plot's width.
const LIST_M: f64 = 32.0;
/// The most parts listed.
const LISTED: usize = 64;

#[derive(Serialize)]
struct NearOne {
    id: u64,
    item: Item,
    away_m: f64,
    paint: u8,
    solid: bool,
    /// What its builder calls it, when named.
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
}

#[derive(Serialize)]
struct Near {
    near: Vec<NearOne>,
}

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
    #[serde(default)]
    clip: Option<ClipBody>,
    /// Place it in front of the body at once, with no click: a file given
    /// is placed where it was given, as soon as it is ready.
    #[serde(default)]
    ahead: bool,
    /// The name the next landmark bears.
    #[serde(default)]
    name: Option<String>,
    /// The landmark the next portal leads to, by its name in the world; 0
    /// leads nowhere.
    #[serde(default)]
    to: Option<u64>,
    /// How wide the next portal's rim is, of its width: 0 is no rim, the
    /// pane alone; a half is the rim to the middle.
    #[serde(default)]
    rim: Option<f32>,
}

/// A sound's record as the seam says it: its file and how long it plays.
#[derive(Deserialize)]
struct ClipBody {
    file: String,
    #[serde(default)]
    millis: u32,
}

impl ClipBody {
    fn clip(&self) -> Option<Clip> {
        Clip::parse(&self.file, self.millis)
    }
}

/// A picture's record as the seam says it.
#[derive(Deserialize)]
struct MediaBody {
    width: u32,
    height: u32,
    /// How long it plays, in milliseconds; a still says none.
    #[serde(default)]
    millis: u32,
    thumb: String,
    low: String,
    full: String,
}

impl MediaBody {
    fn media(&self) -> Option<Media> {
        Media::parse(
            self.width,
            self.height,
            self.millis,
            &self.thumb,
            &self.low,
            &self.full,
        )
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
    /// How much the part shines with its own colour.
    shine: Option<f32>,
    /// How far its light goes, metres; 0 is by its size.
    reach_m: Option<f32>,
    /// Whether what sounds of it is silenced.
    muted: Option<bool>,
    /// How loud it sounds, 0 to 1.
    volume: Option<f32>,
    /// Another picture in the place of the one picked: its width follows
    /// the new aspect, its height stays.
    #[serde(default)]
    media: Option<MediaBody>,
    /// What the landmark picked is called.
    #[serde(default)]
    name: Option<String>,
    /// The landmark the portal picked leads to, by its name in the world; 0
    /// leads nowhere.
    #[serde(default)]
    to: Option<u64>,
    /// How wide the portal picked's rim is, of its width.
    #[serde(default)]
    rim: Option<f32>,
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
    /// Whether the model in hand is still on its way: its file has not
    /// arrived, so what it places is a glass box until it does.
    loading: bool,
    /// The name the next landmark bears, when one was given.
    name: Option<String>,
    /// The landmark the next portal leads to, when one was chosen.
    to: Option<u64>,
    /// How wide the next portal's rim is, of its width.
    rim: f32,
}

/// A part as the panel shows it.
#[derive(Serialize)]
struct Picked {
    id: Option<u64>,
    /// What kind of item it is, by the part's shape.
    #[serde(skip_serializing_if = "Option::is_none")]
    item: Option<Item>,
    #[serde(skip_serializing_if = "Option::is_none")]
    at: Option<[f64; 3]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rotation: Option<[f64; 3]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scale: Option<[f64; 3]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    solid: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shine: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reach_m: Option<f32>,
    /// Whether it is a lamp: a cube in the light finish, or a landmark.
    #[serde(skip_serializing_if = "Option::is_none")]
    lamp: Option<bool>,
    /// Whether it is heard: a video. Its reach is how far.
    #[serde(skip_serializing_if = "Option::is_none")]
    sound: Option<bool>,
    /// Whether what sounds of it is silenced.
    #[serde(skip_serializing_if = "Option::is_none")]
    muted: Option<bool>,
    /// How loud it sounds, 0 to 1.
    #[serde(skip_serializing_if = "Option::is_none")]
    volume: Option<f32>,
    /// What a landmark is called.
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    /// Where a portal leads: the landmark's name in the world, `null` when
    /// it leads nowhere.
    #[serde(skip_serializing_if = "Option::is_none")]
    to: Option<Option<u64>>,
    /// How wide a portal's rim is, of its width.
    #[serde(skip_serializing_if = "Option::is_none")]
    rim: Option<f32>,
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
    /// The sound placed next, once its file is given.
    clip: Option<Clip>,
    /// The model in hand, by the hash of its file: what the model item
    /// places. None until a file is taken.
    model: Option<[u8; 32]>,
    /// Whether the model in hand was still on its way, as last said.
    loading: bool,
    /// The name the next landmark bears. None until one is given.
    name: Option<Name>,
    /// The landmark the next portal leads to. None until one is chosen.
    to: Option<PartId>,
    /// How wide the next portal's rim is, in the part's units.
    rim: u8,
    /// Where the body stood last turn, from the planet's centre: what a
    /// walk through a pane is read from. None right after a leap.
    feet: Option<DVec3>,
    /// Whether a walk through a pane counts: not until the body has stood
    /// clear of every pane since it last leapt, so a portal on the star one
    /// lands on does not send the body straight back.
    armed: bool,
    /// A crossing waiting on the chart: the portal walked through and the
    /// landmark it leads to, one this client does not hold.
    pending: Option<(PartId, PartId)>,
    /// The item in hand is to be placed in front of the body as soon as it
    /// is ready, with no click.
    ahead: bool,
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
            clip: None,
            model: None,
            loading: false,
            name: None,
            to: None,
            rim: Portal::RIM,
            feet: None,
            armed: true,
            pending: None,
            ahead: false,
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

/// Whether two quaternions are one turn: the same, or each other's
/// negative, within a hair.
fn same_turn(a: DQuat, b: DQuat) -> bool {
    a.dot(b).abs() > 1.0 - 1e-9
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

    /// The paint a shape is made of in the colour in hand: a landmark and a
    /// portal are always in the light finish with no edge, lit with their
    /// own light, so only their colour is the hand's; a picture that moves
    /// is a screen, in the light finish, until its shine says otherwise.
    pub(crate) fn paint_of(&self, shape: Shape) -> u8 {
        match shape {
            Shape::Landmark(_) | Shape::Portal(_) => Paint {
                color: self.paint,
                finish: Finish::Light,
                edge: Edge::None,
            }
            .byte(),
            Shape::Image(media) if media.moves() => Paint {
                color: self.paint,
                finish: Finish::Light,
                edge: self.edge,
            }
            .byte(),
            Shape::Cube | Shape::Vox(_) | Shape::Image(_) | Shape::Sound(_) => self.made_of(),
        }
    }

    /// The name the next landmark bears: the one given, or the plain word.
    pub(crate) fn name_in_hand(&self) -> Name {
        self.name
            .unwrap_or_else(|| Name::new("Landmark").expect("a plain word is a name"))
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
        if item.is_none() {
            self.ahead = false;
        }
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
        if let Some(shine) = set.shine {
            part.shine = shine;
        }
        if let Some(reach_m) = set.reach_m {
            part.reach_m = reach_m;
        }
        if let Some(muted) = set.muted {
            part.muted = muted;
        }
        if let Some(volume) = set.volume {
            part.volume = volume.clamp(0.0, 1.0);
        }
        if let Some(body) = &set.media {
            let Some(media) = body.media() else {
                return host.reject("a picture names three asset:// files".into());
            };
            part.shape = Shape::Image(media);
            part.pose.scale[0] = part.pose.scale[2] * media.aspect();
        }
        if let Some(text) = &set.name {
            // A landmark's name is its shape's, and it bears one always;
            // any other part is named for the list, or unnamed again.
            if let Shape::Landmark(_) = part.shape {
                let Some(name) = Name::new(text) else {
                    return refuse(host, Refusal::Name);
                };
                part.shape = Shape::Landmark(name);
            } else if text.trim().is_empty() {
                part.name = None;
            } else {
                let Some(name) = Name::new(text) else {
                    return refuse(host, Refusal::Name);
                };
                part.name = Some(name);
            }
        }
        if set.to.is_some() || set.rim.is_some() {
            let Shape::Portal(mut portal) = part.shape else {
                return refuse(host, Refusal::Nowhere);
            };
            if let Some(to) = set.to {
                portal.to = destination(to);
            }
            if let Some(rim) = set.rim {
                portal.rim = rim_units(rim);
            }
            part.shape = Shape::Portal(portal);
        }
        let (id, seat) = (gizmo.id, gizmo.seat);
        if let (Some(rotation), Some(gizmo)) = (set.rotation, &mut self.picked) {
            // Said angles are shown back as said; a turn made another way
            // is read off the part.
            gizmo.euler = Some(rotation);
        }
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
            part.paint = self.paint_of(part.shape);
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
                loading: self.loading,
                name: self.name.map(|name| name.to_string()),
                to: self.to.map(|to| to.0),
                rim: rim_of_width(self.rim),
            },
        );
    }

    /// Says the parts near the body, the nearest first: those on the seat
    /// the body is over, within a plot's width of its feet in the flat
    /// frame.
    fn say_near(&self, host: &mut Host<'_>) {
        let feet = host.feet();
        let here = [feet.point.u, feet.point.v, feet.height_m / BLOCK_M];
        let mut near: Vec<NearOne> = host
            .parts()
            .all()
            .filter(|(_, seat, _)| *seat == feet.seat)
            .map(|(id, _, part)| {
                let at = part.pose.at;
                let away_m = (0..3)
                    .map(|i| (at[i] - here[i]).powi(2))
                    .sum::<f64>()
                    .sqrt()
                    * BLOCK_M;
                NearOne {
                    id: id.0,
                    item: item_of(part.shape),
                    away_m,
                    paint: part.paint,
                    solid: part.solid,
                    name: name_of(part),
                }
            })
            .filter(|one| one.away_m <= LIST_M)
            .collect();
        near.sort_by(|a, b| a.away_m.total_cmp(&b.away_m));
        near.truncate(LISTED);
        host.emit(NEAR, &Near { near });
    }

    /// Says the landmarks of the world as the chart answered: the nearest
    /// first, each with the place code that takes a body to it.
    fn say_landmarks(&self, charted: &[Landmarked], host: &mut Host<'_>) {
        let landmarks = charted
            .iter()
            .filter_map(|landmark| {
                let Shape::Landmark(name) = landmark.part.shape else {
                    return None;
                };
                Some(LandmarkOne {
                    id: landmark.id,
                    name: name.to_string(),
                    paint: landmark.part.paint,
                    away_m: landmark.away_m,
                    place: host.place_at(landmark.seat, &landmark.part.pose),
                })
            })
            .collect();
        host.emit(LANDMARKS, &Landmarks { landmarks });
    }

    /// Places the item in hand in front of the body, when it is to be and
    /// is ready: a model waits for its file.
    fn place_when_ready(&mut self, host: &mut Host<'_>) {
        if !self.ahead || self.is_loading(host) {
            return;
        }
        self.ahead = false;
        if let Some(item) = self.item {
            self.place_ahead(item, host);
        }
    }

    /// Whether the model in hand is still on its way.
    fn is_loading(&self, host: &Host<'_>) -> bool {
        self.model
            .is_some_and(|hash| host.model_size(hash).is_none())
    }

    fn say_picked(&self, host: &mut Host<'_>) {
        let picked = match &self.picked {
            Some(gizmo) => Picked {
                id: Some(gizmo.id.0),
                item: Some(item_of(gizmo.part.shape)),
                at: Some(gizmo.part.pose.at),
                rotation: Some(
                    gizmo
                        .euler
                        .filter(|said| same_turn(radians(*said), gizmo.part.pose.rotation))
                        .unwrap_or_else(|| degrees(gizmo.part.pose.rotation)),
                ),
                scale: Some(gizmo.part.pose.scale),
                solid: Some(gizmo.part.solid),
                shine: Some(gizmo.part.shine),
                reach_m: Some(gizmo.part.reach_m),
                lamp: Some(client::is_lamp(&gizmo.part)),
                sound: Some(match gizmo.part.shape {
                    Shape::Image(media) => media.sounds(),
                    Shape::Sound(_) => true,
                    _ => false,
                }),
                name: name_of(&gizmo.part),
                muted: Some(gizmo.part.muted),
                volume: Some(if gizmo.part.volume > 0.0 {
                    gizmo.part.volume
                } else {
                    1.0
                }),
                to: match gizmo.part.shape {
                    Shape::Portal(portal) => Some(portal.to.map(|to| to.0)),
                    _ => None,
                },
                rim: match gizmo.part.shape {
                    Shape::Portal(portal) => Some(rim_of_width(portal.rim)),
                    _ => None,
                },
            },
            None => Picked {
                id: None,
                item: None,
                at: None,
                rotation: None,
                scale: None,
                solid: None,
                shine: None,
                reach_m: None,
                lamp: None,
                sound: None,
                muted: None,
                volume: None,
                name: None,
                to: None,
                rim: None,
            },
        };
        host.emit(PICKED, &picked);
    }

    /// One turn with the menu open and no item in hand: a handle dragged
    /// moves the part picked; a click picks a handle, a part, or nothing.
    fn edit_turn(&mut self, turn: &Turn<'_>, host: &mut Host<'_>) {
        let (eye, toward) = turn.eye.sight();
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
            // The gizmo works on the part's body, which may be moving.
            let from = gizmo.on_body(host, eye);
            if gizmo.drag.is_some() {
                if released || turn.interrupted {
                    gizmo.drag = None;
                } else if let Some(frame) =
                    gizmo.dragged(mode_of(self.mode, gizmo), uniform, from, toward)
                    && let Some((seat, pose)) =
                        host.reseat(gizmo.seat.body(), &gizmo.in_world(host, &frame))
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
                let hit = gizmo.handle_hit(mode_of(self.mode, gizmo), from, toward);
                gizmo.hot = hit.map(|(axis, _)| axis);
                if pressed && let Some((axis, _)) = hit {
                    gizmo.press(mode_of(self.mode, gizmo), axis, from, toward);
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
            let id = host.pick(eye, toward).map(|(id, _)| id);
            if id != self.picked.as_ref().map(|gizmo| gizmo.id) {
                self.pick(id, host);
            }
        }
        let marks = self
            .picked
            .as_ref()
            .map(|gizmo| gizmo.marks(host, mode_of(self.mode, gizmo)));
        host.mark(marks.as_deref().unwrap_or(&[]));
    }
}

fn refuse(host: &mut Host<'_>, reason: Refusal) {
    host.emit(REFUSED, &Refused { reason });
}

/// What kind of item a shape is.
/// What a part is called: a landmark by its shape's name, any other by the
/// name its builder gave it, if one.
fn name_of(part: &Part) -> Option<String> {
    match part.shape {
        Shape::Landmark(name) => Some(name.to_string()),
        _ => part.name.map(|name| name.to_string()),
    }
}

/// The handles a part takes: a sound has a place and nothing else, so its
/// handles move it whatever mode is in hand.
fn mode_of(mode: Mode, gizmo: &Gizmo) -> Mode {
    match gizmo.part.shape {
        Shape::Sound(_) => Mode::Move,
        _ => mode,
    }
}

fn item_of(shape: Shape) -> Item {
    match shape {
        Shape::Cube => Item::Cube,
        Shape::Vox(_) => Item::Model,
        Shape::Image(media) if media.kind == ImageKind::Mp4 => Item::Video,
        Shape::Image(_) => Item::Picture,
        Shape::Landmark(_) => Item::Landmark,
        Shape::Sound(_) => Item::Sound,
        Shape::Portal(_) => Item::Portal,
    }
}

/// The landmark a destination on the seam names: 0 is none.
fn destination(to: u64) -> Option<PartId> {
    (to != 0).then_some(PartId(to))
}

/// A rim's width as the seam says it, of the portal's width, in the part's
/// units: none under nothing, the middle at most.
fn rim_units(rim: f32) -> u8 {
    let units = (rim.max(0.0) * f32::from(Portal::RIM_UNITS)).round();
    units.min(f32::from(Portal::RIM_MAX)) as u8
}

/// A rim's width in the part's units, of the portal's width.
fn rim_of_width(rim: u8) -> f32 {
    Portal { to: None, rim }.rim_of_width()
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
                if let Some(Take {
                    item,
                    asset,
                    media,
                    clip,
                    ahead,
                    name,
                    to,
                    rim,
                }) = read(body, host)
                {
                    if item == Some(Item::Sound) {
                        match clip.as_ref().map(ClipBody::clip) {
                            Some(None) => {
                                return host.reject("a sound names an asset:// file".into());
                            }
                            given => self.clip = given.flatten(),
                        }
                    }
                    if let Some(text) = name {
                        let Some(name) = Name::new(&text) else {
                            return refuse(host, Refusal::Name);
                        };
                        self.name = Some(name);
                    }
                    if let Some(to) = to {
                        self.to = destination(to);
                    }
                    if let Some(rim) = rim {
                        self.rim = rim_units(rim);
                    }
                    if matches!(item, Some(Item::Picture | Item::Video)) {
                        match media.as_ref().map(MediaBody::media) {
                            Some(None) => {
                                return host.reject("a picture names three asset:// files".into());
                            }
                            picked => self.picture = picked.flatten(),
                        }
                    }
                    self.take_with(item, asset, host);
                    self.ahead = ahead && self.item.is_some();
                    self.place_when_ready(host);
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
            LIST => self.say_near(host),
            CHART => host.chart(),
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
        // The body moved: through a pane, it is taken where the portal leads.
        self.cross(host);
        // The chart answered: a crossing that waited on it leaps, and the
        // landmarks are said.
        if let Some(charted) = host.take_charted() {
            self.charted(&charted, host);
            self.say_landmarks(&charted, host);
        }
        // The model in hand arrived, or a new one is on its way: the hand
        // says so.
        let loading = self.is_loading(host);
        if loading != self.loading {
            self.loading = loading;
            self.say_hand(host);
            self.place_when_ready(host);
        }
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
        // Placed, the picture is picked, and the pick says what kind it is.
        let said = events(&mut client);
        let picked = said
            .iter()
            .rev()
            .find(|e| e["type"] == "items.picked")
            .unwrap();
        assert_eq!(picked["item"], "picture");

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
        // Another picture in the picked one's place keeps its height and
        // takes the new aspect.
        let square = format!(
            r#"{{"width":1000,"height":1000,"thumb":"asset://{}.webp","low":"asset://{}.webp","full":"asset://{}.png"}}"#,
            hex("04"),
            hex("05"),
            hex("06")
        );
        client.command_json(&format!(r#"{{"type":"items.set","media":{square}}}"#));
        let part = client.parts().all().next().unwrap().2;
        assert!(matches!(part.shape, Shape::Image(media) if media.width == 1000));
        assert!(
            (part.pose.scale[0] - part.pose.scale[2]).abs() < 1e-9,
            "{:?}",
            part.pose.scale
        );
        // The picture placed is picked: a cube beside it is placed off it.
        client.command_json(r#"{"type":"items.take","item":"cube"}"#);
        click(&mut client, [0.3, 0.7]);
        assert_eq!(client.parts().all().count(), 2, "a cube beside the picture");
        // A picture that moves is placed as a screen, whatever the hand's finish.
        let gif = format!(
            r#"{{"width":320,"height":240,"millis":1500,"thumb":"asset://{}.webp","low":"asset://{}.webp","full":"asset://{}.gif"}}"#,
            hex("07"),
            hex("08"),
            hex("09")
        );
        client.command_json(&format!(
            r#"{{"type":"items.take","item":"picture","media":{gif}}}"#
        ));
        click(&mut client, [0.7, 0.7]);
        let (_, _, screen) = client
            .parts()
            .all()
            .find(|(_, _, part)| matches!(part.shape, Shape::Image(media) if media.moves()))
            .expect("the moving picture is placed");
        assert_eq!(voxel::Paint::of(screen.paint).finish, voxel::Finish::Light);
        // A video is taken by its own name, placed as a picture is, and the
        // pick says it is a video.
        let video = format!(
            r#"{{"width":640,"height":360,"millis":4000,"thumb":"asset://{}.webp","low":"asset://{}.webp","full":"asset://{}.mp4"}}"#,
            hex("0a"),
            hex("0b"),
            hex("0c")
        );
        client.command_json(&format!(
            r#"{{"type":"items.take","item":"video","media":{video}}}"#
        ));
        events(&mut client);
        click(&mut client, [0.5, 0.85]);
        let said = events(&mut client);
        let picked = said
            .iter()
            .rev()
            .find(|e| e["type"] == "items.picked")
            .expect("the video placed is picked");
        assert_eq!(picked["item"], "video");
        assert!(
            client
                .parts()
                .all()
                .any(|(_, _, part)| matches!(part.shape, Shape::Image(media) if media.sounds())),
            "the video is placed as a picture that sounds"
        );
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
    fn any_part_is_named_for_the_list_and_unnamed_again() {
        let mut client = client();
        client.command_json(r#"{"type":"items.take","item":"cube"}"#);
        click(&mut client, [0.5, 0.7]);
        events(&mut client);
        client.command_json(r#"{"type":"items.set","name":"  Fountain "}"#);
        let said = events(&mut client);
        let picked = said
            .iter()
            .rev()
            .find(|e| e["type"] == "items.picked")
            .unwrap();
        assert_eq!(picked["name"], "Fountain");
        client.command_json(r#"{"type":"items.list","on":true}"#);
        let said = events(&mut client);
        let near = said
            .iter()
            .rev()
            .find(|e| e["type"] == "items.near")
            .unwrap();
        assert_eq!(near["near"][0]["name"], "Fountain");
        // Unnamed again: nothing said of a name.
        client.command_json(r#"{"type":"items.set","name":""}"#);
        let said = events(&mut client);
        let picked = said
            .iter()
            .rev()
            .find(|e| e["type"] == "items.picked")
            .unwrap();
        assert!(picked.get("name").is_none());
        // Over the bound: refused, the part keeps its name.
        client.command_json(&format!(
            r#"{{"type":"items.set","name":"{}"}}"#,
            "x".repeat(65)
        ));
        assert!(
            events(&mut client)
                .iter()
                .any(|e| e["type"] == "items.refused" && e["reason"] == "name")
        );
    }

    #[test]
    fn a_turn_said_in_degrees_is_shown_back_as_said() {
        let mut client = client();
        client.command_json(r#"{"type":"items.take","item":"cube"}"#);
        click(&mut client, [0.5, 0.7]);
        events(&mut client);
        // Past 90 degrees about y, the one decomposition would say
        // [180, 60, 180]; the panel hears what it said.
        client.command_json(r#"{"type":"items.set","rotation":[0,120,0]}"#);
        let said = events(&mut client);
        let picked = said
            .iter()
            .rev()
            .find(|e| e["type"] == "items.picked")
            .unwrap();
        assert_eq!(picked["rotation"], serde_json::json!([0.0, 120.0, 0.0]));
        client.command_json(r#"{"type":"items.set","rotation":[30,120,0]}"#);
        let said = events(&mut client);
        let picked = said
            .iter()
            .rev()
            .find(|e| e["type"] == "items.picked")
            .unwrap();
        assert_eq!(picked["rotation"], serde_json::json!([30.0, 120.0, 0.0]));
        // Turned another way, the part's own turn is read off it.
        client.command_json(r#"{"type":"items.set","rotation":[0,0,0]}"#);
        events(&mut client);
        let part = client.parts().all().next().unwrap().2;
        assert!(same_turn(part.pose.rotation, DQuat::IDENTITY));
    }

    #[test]
    fn a_sound_given_is_placed_ahead_invisible_and_heard() {
        let mut client = client();
        let hex = "0a".repeat(32);
        // Taken with no file yet, nothing is placed.
        client.command_json(r#"{"type":"items.take","item":"sound","ahead":true}"#);
        client.settled_frame();
        assert_eq!(client.parts().all().count(), 0);
        client.command_json(&format!(
            r#"{{"type":"items.take","item":"sound","clip":{{"file":"asset://{hex}.mp3","millis":4000}},"ahead":true}}"#
        ));
        let frame = client.settled_frame();
        let (_, _, part) = client.parts().all().next().expect("placed ahead");
        assert!(matches!(part.shape, Shape::Sound(clip) if clip.millis == 4000));
        assert!(!part.solid, "nobody collides with a sound");
        assert!(
            frame.models.is_empty() && frame.pictures.is_empty(),
            "nothing to the eye"
        );
        let said = events(&mut client);
        let picked = said
            .iter()
            .rev()
            .find(|e| e["type"] == "items.picked")
            .expect("picked once placed");
        assert_eq!(picked["item"], "sound");
        assert_eq!(picked["sound"], true);
        // Standing beside it, the shell is asked to play it, and it is heard where it stands.
        assert!(
            matches!(client.drain_sounding().as_slice(), [client::Sounding::Open { path, .. }] if path.ends_with(".mp3"))
        );
        assert_eq!(client.sounds().len(), 1);
        // A name that is no file: refused, nothing in hand.
        client.command_json(r#"{"type":"items.take","item":"sound","clip":{"file":"song.mp3"}}"#);
        assert!(
            client
                .drain_events()
                .iter()
                .any(|event| matches!(event, Event::Rejected { .. }))
        );
    }

    #[test]
    fn a_take_ahead_places_in_front_of_the_body_with_no_click() {
        let mut client = client();
        client.command_json(r#"{"type":"items.take","item":"cube","ahead":true}"#);
        assert_eq!(client.parts().all().count(), 1, "placed at once");
        let said = events(&mut client);
        let picked = said
            .iter()
            .rev()
            .find(|e| e["type"] == "items.picked")
            .unwrap();
        assert!(picked["id"].is_number(), "and picked");
        let hand = said
            .iter()
            .rev()
            .find(|e| e["type"] == "items.hand")
            .unwrap();
        assert!(hand["item"].is_null(), "the hand is empty again");
        // In front of the feet, a step or so, on the ground.
        let feet = client.host("test").feet();
        let (_, _, part) = client.parts().all().next().unwrap();
        let [du, dv] = [
            part.pose.at[0] - feet.point.u,
            part.pose.at[1] - feet.point.v,
        ];
        let away = (du * du + dv * dv).sqrt();
        assert!((3.0..8.0).contains(&away), "{away} blocks ahead");
    }

    /// Standing on the ground, an add lands on it; high up, the cube floats
    /// in front of the body at its own height.
    #[test]
    fn a_take_ahead_lands_on_ground_within_reach_and_floats_over_a_drop() {
        let mut client = client();
        let mut input = Input::default();
        // On foot, settled on the ground.
        for _ in 0..30 {
            client.update(1.0 / 60.0, &mut input);
        }
        client.command_json(r#"{"type":"items.take","item":"cube","ahead":true}"#);
        let (_, seat, part) = client
            .parts()
            .all()
            .map(|(id, seat, part)| (id, seat, *part))
            .next()
            .unwrap();
        let under = client.host("test").ground(
            seat,
            topology::SurfacePoint::new(seat.sector(), part.pose.at[0], part.pose.at[1]),
        );
        assert!(
            (part.pose.at[2] - under).abs() < 1.0,
            "stands on the ground: at {} blocks, the ground under it at {under}",
            part.pose.at[2]
        );
        // Ten metres up: in the air on purpose, the cube floats level with
        // the feet.
        client.pose(10.0, 0.0, 4.0);
        client.update(1.0 / 60.0, &mut input);
        client.command_json(r#"{"type":"items.take","item":"cube","ahead":true}"#);
        let feet = client.host("test").feet();
        let (_, _, part) = client.parts().all().last().unwrap();
        assert!(
            (part.pose.at[2] - feet.height_m / BLOCK_M).abs() < 1.0,
            "floats at the feet's height {} blocks, placed at {}",
            feet.height_m / BLOCK_M,
            part.pose.at[2]
        );
        assert!(part.pose.at[2] - under > 10.0, "well over the ground");
    }

    #[test]
    fn a_part_placed_over_another_stands_on_its_top() {
        let mut client = client();
        client.command_json(r#"{"type":"items.take","item":"cube","ahead":true}"#);
        client.command_json(r#"{"type":"items.set","scale":[6,6,6]}"#);
        let (_, _, under) = client.parts().all().next().unwrap();
        let under_top = under.pose.at[2] + 6.0;
        // The line down from over the body's front meets the big cube's
        // top first, and the next cube stands there, not on the ground.
        client.command_json(r#"{"type":"items.take","item":"cube","ahead":true}"#);
        assert_eq!(client.parts().all().count(), 2);
        let (_, _, over) = client
            .parts()
            .all()
            .max_by(|a, b| a.2.pose.at[2].total_cmp(&b.2.pose.at[2]))
            .unwrap();
        assert!(
            (over.pose.at[2] - under_top).abs() < 1e-6,
            "{} on {under_top}",
            over.pose.at[2]
        );
    }

    #[test]
    fn a_landmark_bears_the_name_in_hand_is_a_lamp_and_is_renamed() {
        let mut client = client();
        client.command_json(r#"{"type":"items.paint","paint":11}"#);
        client.command_json(r#"{"type":"items.finish","finish":"glass"}"#);
        client.command_json(
            r#"{"type":"items.take","item":"landmark","name":" Praça ","ahead":true}"#,
        );
        let (_, _, part) = *client.parts().all().collect::<Vec<_>>().first().unwrap();
        assert!(matches!(part.shape, Shape::Landmark(name) if name.as_str() == "Praça"));
        assert!(!part.solid);
        let paint = Paint::of(part.paint);
        assert_eq!(
            (paint.color, paint.finish, paint.edge),
            (11, Finish::Light, Edge::None),
            "always lit, whatever finish is in hand"
        );
        let said = events(&mut client);
        let picked = said
            .iter()
            .rev()
            .find(|e| e["type"] == "items.picked")
            .unwrap();
        assert_eq!(picked["item"], "landmark");
        assert_eq!(picked["name"], "Praça");
        assert_eq!(picked["lamp"], true);
        // Renamed by the panel; a name that is no name is refused.
        client.command_json(r#"{"type":"items.set","name":"Sé"}"#);
        let (_, _, part) = client.parts().all().next().unwrap();
        assert!(matches!(part.shape, Shape::Landmark(name) if name.as_str() == "Sé"));
        client.command_json(r#"{"type":"items.set","name":"   "}"#);
        assert_eq!(events(&mut client).last().unwrap()["reason"], "name");
        // Repainted, it stays lit.
        client.command_json(r#"{"type":"items.paint","paint":2}"#);
        let (_, _, part) = client.parts().all().next().unwrap();
        assert_eq!(Paint::of(part.paint).finish, Finish::Light);
        // With no name given, the plain word.
        client.command_json(r#"{"type":"items.take","item":"landmark","ahead":true}"#);
        assert!(
            client
                .parts()
                .all()
                .any(|(_, _, part)| matches!(part.shape, Shape::Landmark(name) if name.as_str() == "Praça")),
            "the name in hand stays in hand"
        );
    }

    #[test]
    fn the_chart_lists_every_landmark_with_a_place_at_any_level() {
        let mut client = client();
        for name in ["Far", "Near"] {
            client.command_json(&format!(
                r#"{{"type":"items.take","item":"landmark","name":"{name}","ahead":true}}"#
            ));
        }
        // The first placed is set further along than the second.
        let (id, _, part) = client.parts().all().next().unwrap();
        let mut far = *part;
        far.pose.at[0] += 40.0;
        client.command_json(&format!(r#"{{"type":"items.pick","id":{}}}"#, id.0));
        client.command_json(&format!(
            r#"{{"type":"items.set","at":[{},{},{}]}}"#,
            far.pose.at[0], far.pose.at[1], far.pose.at[2]
        ));
        client.command_json(r#"{"type":"items.take","item":"cube","ahead":true}"#);
        events(&mut client);
        client.command_json(r#"{"type":"items.chart"}"#);
        let mut input = Input::default();
        client.update(1.0 / 60.0, &mut input);
        let said = events(&mut client);
        let landmarks = said
            .iter()
            .rev()
            .find(|e| e["type"] == "items.landmarks")
            .expect("the chart is answered in the turn");
        let rows = landmarks["landmarks"].as_array().unwrap();
        let names: Vec<&str> = rows
            .iter()
            .map(|row| row["name"].as_str().unwrap())
            .collect();
        assert_eq!(names, ["Near", "Far"], "the landmarks alone, nearest first");
        assert!(rows[0]["away_m"].as_f64().unwrap() < rows[1]["away_m"].as_f64().unwrap());
        let place = rows[1]["place"].as_str().unwrap();
        assert!(client.go_to(place).is_ok(), "{place} takes a body there");
        // A visitor asks the chart too.
        let mut visitor = Client::new(Recipe::new(1)).unwrap();
        visitor.plug(plugin());
        visitor.rehearse(Level::SignedIn);
        visitor.command_json(r#"{"type":"items.chart"}"#);
        visitor.update(1.0 / 60.0, &mut input);
        let said = events(&mut visitor);
        assert!(said.iter().any(|e| e["type"] == "items.landmarks"));
        assert!(said.iter().all(|e| e["type"] != "items.refused"));
    }

    /// Walks the body forward until the plugin says it crossed, or gives up.
    fn walk_until_crossed(client: &mut Client, frames: usize) -> Option<Value> {
        let mut input = Input::default();
        input.key(Key::Forward, true);
        for _ in 0..frames {
            client.update(1.0 / 60.0, &mut input);
            if let Some(crossed) = events(client)
                .into_iter()
                .find(|e| e["type"] == "items.crossed")
            {
                return Some(crossed);
            }
        }
        None
    }

    #[test]
    fn a_portal_leads_to_the_landmark_in_hand_and_a_walk_through_it_lands_on_the_star() {
        let mut client = client();
        client
            .command_json(r#"{"type":"items.take","item":"landmark","name":"Praça","ahead":true}"#);
        let (star, _, part) = client.parts().all().next().unwrap();
        // The star is set sixty metres along the seat: a leap, never a walk.
        let mut far = *part;
        far.pose.at[1] += 120.0;
        client.command_json(&format!(r#"{{"type":"items.pick","id":{}}}"#, star.0));
        client.command_json(&format!(
            r#"{{"type":"items.set","at":[{},{},{}]}}"#,
            far.pose.at[0], far.pose.at[1], far.pose.at[2]
        ));
        client.command_json(&format!(
            r#"{{"type":"items.take","item":"portal","to":{},"ahead":true}}"#,
            star.0
        ));
        let (portal, _, part) = client
            .parts()
            .all()
            .find(|(id, _, _)| *id != star)
            .expect("a portal placed ahead");
        assert_eq!(
            part.shape,
            Shape::Portal(Portal {
                to: Some(star),
                rim: Portal::RIM
            })
        );
        assert!(!part.solid);
        assert_eq!(part.pose.scale, [8.0, 0.1, 8.0], "four metres across");
        let said = events(&mut client);
        let picked = said
            .iter()
            .rev()
            .find(|e| e["type"] == "items.picked")
            .unwrap();
        assert_eq!(picked["item"], "portal");
        assert_eq!(picked["to"], star.0);
        let hand = said
            .iter()
            .rev()
            .find(|e| e["type"] == "items.hand")
            .unwrap();
        assert_eq!(hand["to"], star.0, "the destination stays in hand");
        // The ring stands in front of the body, facing it: a walk forward
        // goes through its pane and out onto the star, facing its way.
        let crossed = walk_until_crossed(&mut client, 600).expect("walked through the ring");
        assert_eq!(crossed["portal"], portal.0);
        assert_eq!(crossed["to"], star.0);
        let place = crossed["place"].as_str().expect("the place arrived at");
        assert!(place.contains(','), "{place} says which way to face");
        // The body falls into the portal for a moment, the picture drawn
        // into it, then is through: the picture bursts out where it stands.
        let mut input = Input::default();
        let frame = client.update(1.0 / 60.0, &mut input);
        let warp = frame.warp.expect("the picture is drawn into the portal");
        assert!(warp.phase < 0.0, "{warp:?}");
        let before = client.host("items").feet();
        assert!(
            (before.point.v - far.pose.at[1]).abs() > 10.0,
            "not through yet"
        );
        for _ in 0..30 {
            client.update(1.0 / 60.0, &mut input);
        }
        let frame = client.update(1.0 / 60.0, &mut input);
        let warp = frame.warp.expect("the picture bursts out past the portal");
        assert!(warp.phase >= 0.0, "{warp:?}");
        for _ in 0..60 {
            client.update(1.0 / 60.0, &mut input);
        }
        let frame = client.update(1.0 / 60.0, &mut input);
        assert!(frame.warp.is_none(), "the passage is over in a second");
        let feet = client.host("items").feet();
        assert!(
            (feet.point.u - far.pose.at[0]).abs() < 2.0
                && (feet.point.v - far.pose.at[1]).abs() < 2.0,
            "landed at {:?}, the star at {:?}",
            feet.point,
            far.pose.at
        );
        // Landed, the body walks on without being sent anywhere.
        assert!(walk_until_crossed(&mut client, 60).is_none());
    }

    #[test]
    fn a_portal_leading_nowhere_says_so_and_the_body_stays() {
        let mut client = client();
        client.command_json(r#"{"type":"items.take","item":"portal","ahead":true}"#);
        let (portal, _, part) = client.parts().all().next().unwrap();
        assert_eq!(
            part.shape,
            Shape::Portal(Portal {
                to: None,
                rim: Portal::RIM
            })
        );
        let said = events(&mut client);
        let picked = said
            .iter()
            .rev()
            .find(|e| e["type"] == "items.picked")
            .unwrap();
        assert!(picked["to"].is_null(), "{picked}");
        let before = client.here().place(client.sphere().blocks());
        let crossed = walk_until_crossed(&mut client, 600).expect("walked through the ring");
        assert_eq!(crossed["portal"], portal.0);
        assert!(
            crossed["to"].is_null() && crossed["place"].is_null(),
            "{crossed}"
        );
        let after = client.here().place(client.sphere().blocks());
        assert_eq!(
            &before[..6],
            &after[..6],
            "a walk, no leap: {before} to {after}"
        );
        // Aimed by the panel at a landmark, the portal leads there; aiming a
        // part that is no portal is refused.
        client.command_json(r#"{"type":"items.take","item":"landmark","ahead":true}"#);
        let (star, _, _) = client
            .parts()
            .all()
            .find(|(_, _, part)| matches!(part.shape, Shape::Landmark(_)))
            .unwrap();
        client.command_json(&format!(r#"{{"type":"items.set","to":{}}}"#, star.0));
        assert_eq!(events(&mut client).last().unwrap()["reason"], "nowhere");
        client.command_json(&format!(r#"{{"type":"items.pick","id":{}}}"#, portal.0));
        client.command_json(&format!(r#"{{"type":"items.set","to":{}}}"#, star.0));
        assert_eq!(
            client.parts().get(portal).unwrap().1.shape,
            Shape::Portal(Portal {
                to: Some(star),
                rim: Portal::RIM
            })
        );
        let picked = events(&mut client)
            .into_iter()
            .rev()
            .find(|e| e["type"] == "items.picked")
            .unwrap();
        assert_eq!(picked["to"], star.0);
        // The rim is the hand's: none at all, or to the middle at most.
        client.command_json(r#"{"type":"items.set","rim":0}"#);
        assert!(matches!(
            client.parts().get(portal).unwrap().1.shape,
            Shape::Portal(Portal { rim: 0, .. })
        ));
        assert_eq!(
            events(&mut client)
                .into_iter()
                .rev()
                .find(|e| e["type"] == "items.picked")
                .unwrap()["rim"],
            0.0
        );
        client.command_json(r#"{"type":"items.set","rim":0.9}"#);
        assert!(matches!(
            client.parts().get(portal).unwrap().1.shape,
            Shape::Portal(Portal {
                rim: Portal::RIM_MAX,
                ..
            })
        ));
        client.command_json(r#"{"type":"items.take","item":"portal","rim":0.25,"ahead":true}"#);
        let (_, _, part) = client.parts().all().last().unwrap();
        assert!(
            matches!(part.shape, Shape::Portal(Portal { rim: 50, .. })),
            "{:?}",
            part.shape
        );
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
