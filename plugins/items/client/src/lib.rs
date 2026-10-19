//! The items plugin's client half: the menu of things to place in the
//! world, the item in hand, and the click that places it as a part
//! (DECISIONS 111). Building is the cells; this is everything placed that
//! is no cell.
//!
//! An item is what the menu offers: a cube in a paint, and a picture, which
//! is taken with the record of its file. A model from a file is an item of
//! its own, a shape of part, and enters here as a kind with the words it
//! needs. The parts are the core's: this plugin asks its host to place one
//! and holds no part.
//!
//! The seam, under `items.`: the commands `take`, `paint`, `finish`, `edge`
//! and `state`; the events `hand` and `refused`.

mod place;

use client::{Host, KeyAsk, Media, Plugin, Turn};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use voxel::{Edge, Finish, Paint};

pub use items_world::{NAME, VERSION};

/// What the menu offers to place.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Item {
    /// A cube of a metre, in the paint in hand.
    Cube,
    /// A picture a metre tall in its own aspect, facing the eye, a pane
    /// nobody collides with: a screen in a light finish, a print in any
    /// other. Taken with its `media`, what the owner's client made of the
    /// file: its size in texels and three asset names, the original, a low
    /// version and a thumb.
    Picture,
}

/// Why an item was not handed over, or a part not placed. A front end says
/// it in its own words.
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

/// `items.take`: take an item in hand, or `null` to put it down. A picture
/// rides with its `media`, or with none while its file is still to pick.
pub const TAKE: &str = "take";
/// `items.paint`: pick a colour of the palette for what is placed next.
pub const PAINT: &str = "paint";
/// `items.finish`: how what is placed next takes the light.
pub const FINISH: &str = "finish";
/// `items.edge`: the line around each side of what is placed next.
pub const EDGE: &str = "edge";
/// `items.state`: say what is in hand, for a layer that just mounted.
pub const STATE: &str = "state";
/// `items.hand`: the item in hand, `null` when none, and the paint in hand.
pub const HAND: &str = "hand";
/// `items.refused`: why what was asked was not done.
pub const REFUSED: &str = "refused";

#[derive(Deserialize)]
struct Take {
    item: Option<Item>,
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
    item: Option<Item>,
    paint: u8,
    finish: Finish,
    edge: Edge,
}

#[derive(Serialize)]
struct Refused {
    reason: Refusal,
}

mod key {
    /// Take the item taken last, or put the item in hand down.
    pub const ITEMS: &str = "items";
    /// Put the item in hand down.
    pub const CANCEL: &str = "cancel";
}

/// The colours a hand can pick: as many as the world's palette holds.
const PAINTS: u8 = client::PALETTE.len() as u8;

/// The item in hand and the paint it is placed in.
pub struct Items {
    item: Option<Item>,
    /// The item taken last, which the key takes again.
    last: Item,
    paint: u8,
    finish: Finish,
    edge: Edge,
    /// The picture placed next, once its file is picked.
    picture: Option<Media>,
    /// Whether the pointer's button was down last turn.
    using: bool,
}

impl Default for Items {
    fn default() -> Self {
        Items {
            item: None,
            last: Item::Cube,
            paint: 0,
            finish: Finish::Matte,
            edge: Edge::None,
            picture: None,
            using: false,
        }
    }
}

/// The plugin as a shell plugs it in.
pub fn plugin() -> Box<dyn Plugin> {
    Box::new(Items::default())
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

    /// Takes an item in hand or puts it down. With one in hand the pointer
    /// is this plugin's.
    fn take(&mut self, item: Option<Item>, host: &mut Host<'_>) {
        if item.is_some() && !host.may_change() {
            return refuse(host, Refusal::Level);
        }
        self.item = item;
        if let Some(item) = item {
            self.last = item;
        }
        host.point(item.is_some());
        self.say_hand(host);
    }

    fn say_hand(&self, host: &mut Host<'_>) {
        host.emit(
            HAND,
            &Hand {
                item: self.item,
                paint: self.paint,
                finish: self.finish,
                edge: self.edge,
            },
        );
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
            TAKE => {
                if let Some(Take { item, media }) = read(body, host) {
                    if item == Some(Item::Picture) {
                        match media.as_ref().map(MediaBody::media) {
                            Some(None) => {
                                return host.reject("a picture names three asset:// files".into());
                            }
                            picked => self.picture = picked.flatten(),
                        }
                    }
                    self.take(item, host);
                }
            }
            PAINT => {
                if let Some(Painted { paint }) = read(body, host) {
                    self.paint = paint.min(PAINTS - 1);
                    self.say_hand(host);
                }
            }
            FINISH => {
                if let Some(Finished { finish }) = read(body, host) {
                    self.finish = finish;
                    self.say_hand(host);
                }
            }
            EDGE => {
                if let Some(Edged { edge }) = read(body, host) {
                    self.edge = edge;
                    self.say_hand(host);
                }
            }
            STATE => self.say_hand(host),
            _ => host.reject(format!("`{kind}` is no command of items")),
        }
    }

    fn keys(&self) -> Vec<KeyAsk> {
        vec![
            KeyAsk::pressed(key::ITEMS, "KeyI"),
            KeyAsk::pressed(key::CANCEL, "Escape"),
        ]
    }

    fn key(&mut self, name: &str, host: &mut Host<'_>) {
        match name {
            key::ITEMS => {
                let item = match self.item {
                    Some(_) => None,
                    None => Some(self.last),
                };
                self.take(item, host);
            }
            key::CANCEL if self.item.is_some() => self.take(None, host),
            _ => {}
        }
    }

    /// One turn with an item in hand: a click places it.
    fn turn(&mut self, turn: &Turn<'_>, host: &mut Host<'_>) {
        let pressed = turn.using && !self.using && !turn.interrupted;
        self.using = turn.using;
        if pressed && let Some(item) = self.item {
            self.place(item, &turn.eye, host);
        }
    }

    fn rest(&mut self, host: &mut Host<'_>) {
        self.using = false;
        if self.item.is_some() {
            self.take(None, host);
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

    #[test]
    fn a_click_with_a_cube_in_hand_places_a_part_in_the_paint_in_hand() {
        let mut client = client();
        client.command_json(r#"{"type":"items.paint","paint":11}"#);
        client.command_json(r#"{"type":"items.take","item":"cube"}"#);
        let hand = events(&mut client);
        assert_eq!(hand.last().unwrap()["item"], "cube");
        assert!(client.pointing());
        let mut input = Input::default();
        input.pointer = Some([0.5, 0.7]);
        input.key(Key::Use, true);
        client.update(1.0 / 60.0, &mut input);
        input.key(Key::Use, false);
        client.update(1.0 / 60.0, &mut input);
        let placed: Vec<_> = client.parts().all().collect();
        assert_eq!(placed.len(), 1);
        assert_eq!(placed[0].2.paint, 11);
        assert!(placed[0].2.solid);
        // Put down, the pointer goes back and nothing more is placed.
        client.command_json(r#"{"type":"items.take","item":null}"#);
        assert!(!client.pointing());
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
        let mut input = Input::default();
        input.pointer = Some([0.5, 0.7]);
        let click = |client: &mut Client, input: &mut Input| {
            input.key(Key::Use, true);
            client.update(1.0 / 60.0, input);
            input.key(Key::Use, false);
            client.update(1.0 / 60.0, input);
        };
        click(&mut client, &mut input);
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
        click(&mut client, &mut input);

        let frame = client.settled_frame();
        assert_eq!(frame.models.len(), 0, "a picture is no model");
        assert_eq!(frame.pictures.len(), 1);
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
        client.command_json(r#"{"type":"items.take","item":"cube"}"#);
        click(&mut client, &mut input);
        let frame = client.settled_frame();
        assert_eq!(
            (frame.models.len(), frame.pictures.len()),
            (1, 1),
            "a cube beside the picture"
        );
    }

    #[test]
    fn a_visitor_takes_nothing() {
        let mut client = Client::new(Recipe::new(1)).unwrap();
        client.plug(plugin());
        client.rehearse(Level::SignedIn);
        client.command_json(r#"{"type":"items.take","item":"cube"}"#);
        let said = events(&mut client);
        assert_eq!(said.last().unwrap()["reason"], "level");
        assert!(!client.pointing());
    }
}
