//! Input as the controller sees it: intent, not key codes.
//!
//! Platform shells translate their own key and pointer events into [`Key`]s
//! and deltas. Bindings live in the shells; meaning lives here.
//!
//! A plugin's keys are its own (DECISIONS 106): it asks for each by name, as
//! a [`KeyAsk`], and a shell hands over the keys of its keyboard that a
//! plugin asked for, by the name the web gives them.

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
    /// The pointer's button while a plugin has the pointer: what its hand
    /// does is the plugin's.
    Use,
}

/// What is held with a key as it goes down.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub struct Chord {
    /// Command on a Mac, Control elsewhere.
    pub command: bool,
    pub shift: bool,
}

/// A key a plugin asks for by name (DECISIONS 106). It is data: a shell with
/// a keyboard binds it by `code`, and one without binds the name to what it
/// has.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct KeyAsk {
    /// The name the plugin hears it by. Two keys may share one.
    pub name: &'static str,
    /// The key on a keyboard, as the web names it: `KeyB`, `Digit1`,
    /// `AltLeft`, `Escape`.
    pub code: &'static str,
    /// What is held with it. A plain key is heard whatever the shift.
    pub chord: Chord,
    /// Read while it is held, in the plugin's turn. Otherwise it is heard
    /// once, as it goes down.
    pub held: bool,
}

impl KeyAsk {
    /// A key heard once as it goes down, with nothing held.
    pub const fn pressed(name: &'static str, code: &'static str) -> KeyAsk {
        KeyAsk {
            name,
            code,
            chord: Chord {
                command: false,
                shift: false,
            },
            held: false,
        }
    }

    /// A key read while it is held.
    pub const fn held(name: &'static str, code: &'static str) -> KeyAsk {
        KeyAsk {
            held: true,
            ..KeyAsk::pressed(name, code)
        }
    }

    /// A key heard once as it goes down with command held, and with shift
    /// or without it.
    pub const fn command(name: &'static str, code: &'static str, shift: bool) -> KeyAsk {
        KeyAsk {
            chord: Chord {
                command: true,
                shift,
            },
            ..KeyAsk::pressed(name, code)
        }
    }

    /// Whether a key that went down with a chord is this one.
    pub(crate) fn is(&self, code: &str, chord: Chord) -> bool {
        self.code == code
            && self.chord.command == chord.command
            && (!self.chord.command || self.chord.shift == chord.shift)
    }
}

/// Input gathered by a shell since the last update.
#[derive(Clone, Debug, Default)]
pub struct Input {
    held: [bool; 8],
    /// One shot keys pressed since the last update.
    pressed: Vec<Key>,
    /// The keys of a keyboard that are down, as the web names them: what a
    /// plugin may have asked to read held.
    codes_held: Vec<String>,
    /// Those that went down since the last update, each with what was held
    /// with it.
    codes_pressed: Vec<(String, Chord)>,
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

    /// Records a transition of a key of a keyboard by the name the web gives
    /// it, with what is held with it: what the plugins hear. A shell passes
    /// the keys [`crate::Client::asks`] for as they go down, leaving out the
    /// repeats of a key held, and every key as it comes up.
    pub fn code(&mut self, code: &str, chord: Chord, down: bool) {
        self.hold(code, down);
        if down {
            self.codes_pressed.push((code.to_owned(), chord));
        }
    }

    /// Says whether a key of a keyboard is down, and nothing went down: how
    /// a shell keeps a held key in step with what a pointer says is held
    /// with it. Repeats are harmless.
    pub fn hold(&mut self, code: &str, down: bool) {
        let at = self.codes_held.iter().position(|held| held == code);
        match (down, at) {
            (true, None) => self.codes_held.push(code.to_owned()),
            (false, Some(at)) => {
                self.codes_held.swap_remove(at);
            }
            _ => {}
        }
    }

    pub(crate) fn code_held(&self, code: &str) -> bool {
        self.codes_held.iter().any(|held| held == code)
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
        self.codes_held.clear();
        self.interrupted = true;
    }

    /// Takes the one shot input, leaving held keys in place.
    pub(crate) fn take_frame(&mut self) -> Taken {
        let taken = Taken {
            pressed: core::mem::take(&mut self.pressed),
            codes: core::mem::take(&mut self.codes_pressed),
            look: self.look,
            zoom: self.zoom,
            interrupted: self.interrupted,
        };
        self.look = [0.0; 2];
        self.zoom = 0.0;
        self.interrupted = false;
        taken
    }
}

/// The one shot input of a frame.
pub(crate) struct Taken {
    pub pressed: Vec<Key>,
    /// Keys of a keyboard that went down, each with what was held with it.
    pub codes: Vec<(String, Chord)>,
    pub look: [f32; 2],
    pub zoom: f32,
    /// Whether the shell let everything go.
    pub interrupted: bool,
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
        Key::ToggleMode | Key::NewSeed | Key::NextAvatar => None,
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
        assert!(input.take_frame().interrupted);
        assert!(!input.take_frame().interrupted);
    }

    #[test]
    fn a_key_of_the_keyboard_is_heard_once_and_read_while_held() {
        let mut input = Input::default();
        let command = Chord {
            command: true,
            shift: false,
        };
        // A key that never came up, as under a held command, is heard again.
        input.code("KeyZ", command, true);
        input.code("KeyZ", command, true);
        input.code("AltLeft", Chord::default(), true);
        assert_eq!(input.take_frame().codes.len(), 3);
        assert!(input.code_held("AltLeft"));
        input.code("AltLeft", Chord::default(), false);
        assert!(!input.code_held("AltLeft"));
        // Held in step with a pointer, nothing went down.
        input.hold("AltLeft", true);
        input.hold("AltLeft", true);
        assert!(input.code_held("AltLeft"));
        assert!(input.take_frame().codes.is_empty());
        // Letting everything go lets these go too.
        input.release_all();
        assert!(!input.code_held("KeyZ"));
    }

    #[test]
    fn a_plain_key_is_heard_whatever_the_shift_and_a_chord_exactly() {
        let plain = KeyAsk::pressed("build", "KeyB");
        let shift = Chord {
            command: false,
            shift: true,
        };
        assert!(plain.is("KeyB", Chord::default()) && plain.is("KeyB", shift));
        let undo = KeyAsk::command("undo", "KeyZ", false);
        let redo = KeyAsk::command("redo", "KeyZ", true);
        let with = |shift| Chord {
            command: true,
            shift,
        };
        assert!(undo.is("KeyZ", with(false)) && !undo.is("KeyZ", with(true)));
        assert!(redo.is("KeyZ", with(true)) && !redo.is("KeyZ", with(false)));
        assert!(!undo.is("KeyZ", Chord::default()) && !plain.is("KeyB", with(false)));
    }
}
