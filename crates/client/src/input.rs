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
    /// Start building with the last tool, or stop.
    Build,
    /// Take a tool, which starts building if it was not.
    Create,
    Delete,
    Paint,
    /// The pointer's button while building: held down, a stroke is drawn.
    Use,
    /// Each time it goes down, the stroke turns to the next of the three
    /// layers through its start: on its side, then standing one way, then
    /// the other.
    Turn,
    /// Drop the stroke being drawn, or else stop building.
    Cancel,
    /// Take back the last stroke, or put it back.
    Undo,
    Redo,
}

/// Input gathered by a shell since the last update.
#[derive(Clone, Debug, Default)]
pub struct Input {
    held: [bool; 9],
    /// One shot keys pressed since the last update.
    pressed: Vec<Key>,
    /// Whether the shell let everything go since the last update.
    interrupted: bool,
    /// Pointer motion in pixels, x right, y down.
    pub look: [f32; 2],
    /// Wheel motion in lines, positive zooms in.
    pub zoom: f32,
    /// Where the pointer is, as fractions of the view from the top left.
    /// `None` while it is captured: then it aims through the middle.
    pub pointer: Option<[f32; 2]>,
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

    /// Releases everything, for when the window loses focus. What a held key
    /// was in the middle of is dropped, never landed: a stroke half drawn
    /// when the window blurs goes the way of Escape, not of a click let go.
    pub fn release_all(&mut self) {
        self.held = Default::default();
        self.interrupted = true;
    }

    /// Takes the one shot input, leaving held keys in place.
    pub(crate) fn take_frame(&mut self) -> (Vec<Key>, [f32; 2], f32, bool) {
        let out = (
            core::mem::take(&mut self.pressed),
            self.look,
            self.zoom,
            self.interrupted,
        );
        self.look = [0.0; 2];
        self.zoom = 0.0;
        self.interrupted = false;
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
        Key::Use => Some(7),
        Key::Turn => Some(8),
        Key::ToggleMode
        | Key::NewSeed
        | Key::NextAvatar
        | Key::Build
        | Key::Create
        | Key::Delete
        | Key::Paint
        | Key::Cancel
        | Key::Undo
        | Key::Redo => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn losing_focus_lets_go_and_interrupts_once() {
        let mut input = Input::default();
        input.key(Key::Use, true);
        input.release_all();
        assert!(!input.held(Key::Use));
        let (_, _, _, interrupted) = input.take_frame();
        assert!(interrupted);
        let (_, _, _, interrupted) = input.take_frame();
        assert!(!interrupted);
    }
}
