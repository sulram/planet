//! The items plugin's client half: the menu of things to place in the
//! world, the item in hand, and the click that places it as a part
//! (DECISIONS 111). Building is the cells; this is everything placed that
//! is no cell.
//!
//! An item is what the menu offers: a cube in a paint today. A model from a
//! file and a picture are items of their own, each a shape of part, and
//! each enters here as a kind with the words it needs. The parts are the
//! core's: this plugin asks its host to place one and holds no part.
//!
//! The seam, under `items.`: the commands `take`, `paint`, `finish`, `edge`
//! and `state`; the events `hand` and `refused`.

mod place;

use client::{Host, KeyAsk, Plugin, Turn};
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

/// `items.take`: take an item in hand, or `null` to put it down.
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
                if let Some(Take { item }) = read(body, host) {
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
        client.set_aspect(16.0 / 9.0);
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
