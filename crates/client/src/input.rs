//! Input as the controller sees it: intent, not key codes.
//!
//! Platform shells translate their own key and pointer events into [`Key`]s
//! and deltas. Bindings live in the shells; meaning lives here.

/// A held or pressed control.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Key {
    Forward,
    Back,
    Left,
    Right,
    /// Jump on foot, rise in flight.
    Up,
    /// Sink in flight.
    Down,
    Sprint,
    /// Switch between walking and flying.
    ToggleMode,
    /// Explore another planet: a fresh random seed.
    NewSeed,
    /// Wear the next avatar on offer.
    NextAvatar,
}

/// Input gathered by a shell since the last update.
#[derive(Clone, Debug, Default)]
pub struct Input {
    held: [bool; 7],
    /// One shot keys pressed since the last update.
    pressed: Vec<Key>,
    /// Pointer motion in pixels, x right, y down.
    pub look: [f32; 2],
    /// Wheel motion in lines, positive zooms in.
    pub zoom: f32,
}

impl Input {
    /// Records a key transition. Repeats are harmless.
    pub fn key(&mut self, key: Key, down: bool) {
        match held_slot(key) {
            Some(slot) => self.held[slot] = down,
            None if down => self.pressed.push(key),
            None => {}
        }
    }

    pub fn held(&self, key: Key) -> bool {
        held_slot(key).is_some_and(|slot| self.held[slot])
    }

    /// `[right, forward]`, each `-1..=1`.
    pub fn movement(&self) -> [f32; 2] {
        let axis = |pos: Key, neg: Key| f32::from(self.held(pos)) - f32::from(self.held(neg));
        [axis(Key::Right, Key::Left), axis(Key::Forward, Key::Back)]
    }

    /// Releases everything, for when the window loses focus.
    pub fn release_all(&mut self) {
        self.held = Default::default();
    }

    /// Takes the one shot input, leaving held keys in place.
    pub(crate) fn take_frame(&mut self) -> (Vec<Key>, [f32; 2], f32) {
        let out = (core::mem::take(&mut self.pressed), self.look, self.zoom);
        self.look = [0.0; 2];
        self.zoom = 0.0;
        out
    }
}

fn held_slot(key: Key) -> Option<usize> {
    match key {
        Key::Forward => Some(0),
        Key::Back => Some(1),
        Key::Left => Some(2),
        Key::Right => Some(3),
        Key::Up => Some(4),
        Key::Down => Some(5),
        Key::Sprint => Some(6),
        Key::ToggleMode | Key::NewSeed | Key::NextAvatar => None,
    }
}
