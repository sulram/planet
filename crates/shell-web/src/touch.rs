//! Fingers on the canvas, as a phone is held: a finger put down on the left
//! half opens a stick where it lands and walks, one on the right looks, and
//! a second one beside it pinches the eye nearer or farther. What they come
//! to reaches the client through [`Input`], as a key's does: the client never
//! knows a finger from a key.
//!
//! Plain logic over CSS pixels from the canvas's top left, so it is built
//! and tested on every target; the listeners in `web` feed it.

use client::Input;

/// How far a stick reaches, in CSS pixels: pulled past it, its centre
/// follows the finger, so the way back is never longer than the reach.
const REACH_PX: f32 = 48.0;
/// How far a finger moves before its stick wakes: one that only rests on
/// the glass does not walk.
const WAKE_PX: f32 = 3.0;
/// How much faster a finger dragged looks than a mouse moved as far: a
/// thumb across half a phone turns the eye about a right angle.
const LOOK_GAIN: f32 = 3.0;

/// The fingers on the canvas, by pointer id.
#[derive(Default, Debug)]
pub(crate) struct Fingers {
    stick: Option<Stick>,
    /// The finger that looks.
    look: Option<Finger>,
    /// A second finger beside the one that looks: the two pinch, and
    /// neither looks while they do.
    pinch: Option<Finger>,
    /// The stick woke, moved or went since the page last heard of it.
    moved: bool,
}

#[derive(Clone, Copy, Debug)]
struct Finger {
    id: i32,
    at: [f32; 2],
}

#[derive(Clone, Copy, Debug)]
struct Stick {
    id: i32,
    center: [f32; 2],
    at: [f32; 2],
    /// The finger has moved past [`WAKE_PX`]: the stick walks and is drawn.
    awake: bool,
}

impl Stick {
    /// `[right, forward]`, no longer than one.
    fn points(&self) -> [f32; 2] {
        let [dx, dy] = sub(self.at, self.center);
        [dx / REACH_PX, -dy / REACH_PX]
    }

    /// Drags the centre after a finger gone past the reach.
    fn follow(&mut self) {
        let away = sub(self.at, self.center);
        let length = len(away);
        if length > REACH_PX {
            let back = REACH_PX / length;
            self.center = [self.at[0] - away[0] * back, self.at[1] - away[1] * back];
        }
    }
}

impl Fingers {
    /// A finger goes down at `at` on a canvas `width` pixels wide. The first
    /// on the left half holds the stick; any other looks, and a second one
    /// that does pinches. Says whether the finger is held: a third that
    /// would only look is left alone.
    pub fn down(&mut self, id: i32, at: [f32; 2], width: f32) -> bool {
        let finger = Finger { id, at };
        if self.stick.is_none() && at[0] < width / 2.0 {
            self.stick = Some(Stick {
                id,
                center: at,
                at,
                awake: false,
            });
        } else if self.look.is_none() {
            self.look = Some(finger);
        } else if self.pinch.is_none() {
            self.pinch = Some(finger);
        } else {
            return false;
        }
        true
    }

    /// A finger moves to `at`. Says whether it is one of these.
    pub fn moved(&mut self, id: i32, at: [f32; 2], input: &mut Input) -> bool {
        if let Some(stick) = self.stick.as_mut().filter(|stick| stick.id == id) {
            stick.at = at;
            stick.awake |= len(sub(at, stick.center)) > WAKE_PX;
            if stick.awake {
                stick.follow();
                input.stick = stick.points();
                self.moved = true;
            }
            return true;
        }
        match (self.look.as_mut(), self.pinch.as_mut()) {
            (Some(look), Some(pinch)) if look.id == id || pinch.id == id => {
                let before = len(sub(look.at, pinch.at));
                if look.id == id {
                    look.at = at;
                } else {
                    pinch.at = at;
                }
                let after = len(sub(look.at, pinch.at));
                if before > 0.0 {
                    input.zoom_by(after / before);
                }
                true
            }
            (Some(look), None) if look.id == id => {
                let [dx, dy] = sub(at, look.at);
                input.look[0] += dx * LOOK_GAIN;
                input.look[1] += dy * LOOK_GAIN;
                look.at = at;
                true
            }
            _ => false,
        }
    }

    /// A finger lifts, or the browser takes it back. Says whether it was
    /// one of these.
    pub fn up(&mut self, id: i32, input: &mut Input) -> bool {
        if let Some(stick) = self.stick.filter(|stick| stick.id == id) {
            self.moved |= stick.awake;
            self.stick = None;
            input.stick = [0.0; 2];
        } else if self.look.is_some_and(|look| look.id == id) {
            // The one left of a pinch looks on from where it is.
            self.look = self.pinch.take();
        } else if self.pinch.is_some_and(|pinch| pinch.id == id) {
            self.pinch = None;
        } else {
            return false;
        }
        true
    }

    /// Lets every finger go, for when the page loses focus.
    pub fn release(&mut self, input: &mut Input) {
        self.moved |= self.stick.is_some_and(|stick| stick.awake);
        *self = Fingers {
            moved: self.moved,
            ..Fingers::default()
        };
        input.stick = [0.0; 2];
    }

    /// The stick as the page draws it, `stick` in the shell's own words,
    /// when it changed since the last time this was asked: where its
    /// centre is, where the finger is and how far it reaches, or `null`
    /// while there is none awake.
    pub fn told(&mut self) -> Option<String> {
        if !std::mem::take(&mut self.moved) {
            return None;
        }
        Some(match self.stick.filter(|stick| stick.awake) {
            Some(Stick { center, at, .. }) => format!(
                r#"{{"type":"stick","stick":{{"center":[{:.1},{:.1}],"at":[{:.1},{:.1}],"reach":{REACH_PX}}}}}"#,
                center[0], center[1], at[0], at[1]
            ),
            None => r#"{"type":"stick","stick":null}"#.to_owned(),
        })
    }
}

fn sub(a: [f32; 2], b: [f32; 2]) -> [f32; 2] {
    [a[0] - b[0], a[1] - b[1]]
}

fn len([x, y]: [f32; 2]) -> f32 {
    (x * x + y * y).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    const WIDTH: f32 = 400.0;

    #[test]
    fn a_stick_wakes_past_a_few_pixels_and_walks_where_it_points() {
        let (mut fingers, mut input) = (Fingers::default(), Input::default());
        assert!(fingers.down(1, [100.0, 600.0], WIDTH));
        fingers.moved(1, [102.0, 600.0], &mut input);
        assert_eq!(input.stick, [0.0; 2]);
        assert_eq!(fingers.told(), None);
        // Up the screen is forward.
        fingers.moved(1, [100.0, 600.0 - REACH_PX / 2.0], &mut input);
        assert_eq!(input.stick, [0.0, 0.5]);
        assert!(!input.sprinting());
        assert!(
            fingers
                .told()
                .is_some_and(|said| said.contains(r#""center":[100.0,600.0]"#))
        );
        assert_eq!(fingers.told(), None);
        assert!(fingers.up(1, &mut input));
        assert_eq!(input.stick, [0.0; 2]);
        assert_eq!(
            fingers.told().as_deref(),
            Some(r#"{"type":"stick","stick":null}"#)
        );
    }

    #[test]
    fn a_stick_pulled_past_its_reach_drags_its_centre_and_runs() {
        let (mut fingers, mut input) = (Fingers::default(), Input::default());
        fingers.down(1, [100.0, 600.0], WIDTH);
        fingers.moved(1, [100.0 + 3.0 * REACH_PX, 600.0], &mut input);
        assert_eq!(input.stick, [1.0, 0.0]);
        assert!(input.sprinting());
        // Back by the reach is back at the centre, which followed.
        fingers.moved(1, [100.0 + 2.0 * REACH_PX, 600.0], &mut input);
        assert_eq!(input.stick, [0.0, 0.0]);
    }

    #[test]
    fn the_right_half_looks_and_two_fingers_there_pinch() {
        let (mut fingers, mut input) = (Fingers::default(), Input::default());
        fingers.down(1, [300.0, 400.0], WIDTH);
        fingers.moved(1, [310.0, 395.0], &mut input);
        assert_eq!(input.look, [10.0 * LOOK_GAIN, -5.0 * LOOK_GAIN]);
        // A second finger pinches with the first: spread twice as far, the
        // eye comes twice as near, and neither looks meanwhile.
        fingers.down(2, [310.0, 495.0], WIDTH);
        input.look = [0.0; 2];
        fingers.moved(2, [310.0, 595.0], &mut input);
        assert_eq!(input.look, [0.0; 2]);
        let mut twice = Input::default();
        twice.zoom_by(2.0);
        assert!((input.zoom - twice.zoom).abs() < 1e-5);
        // The first lifts: the second looks on from where it is.
        fingers.up(1, &mut input);
        fingers.moved(2, [320.0, 595.0], &mut input);
        assert_eq!(input.look, [10.0 * LOOK_GAIN, 0.0]);
    }

    #[test]
    fn a_stick_held_leaves_the_left_half_to_look_and_a_third_finger_alone() {
        let (mut fingers, mut input) = (Fingers::default(), Input::default());
        fingers.down(1, [50.0, 600.0], WIDTH);
        assert!(fingers.down(2, [60.0, 300.0], WIDTH));
        fingers.moved(2, [70.0, 300.0], &mut input);
        assert_eq!(input.look[0], 10.0 * LOOK_GAIN);
        assert!(fingers.down(3, [300.0, 300.0], WIDTH));
        assert!(!fingers.down(4, [350.0, 300.0], WIDTH));
        assert!(!fingers.moved(4, [360.0, 300.0], &mut input));
        assert!(!fingers.up(4, &mut input));
    }

    #[test]
    fn letting_go_of_the_page_lets_the_stick_go_and_says_so() {
        let (mut fingers, mut input) = (Fingers::default(), Input::default());
        fingers.down(1, [50.0, 600.0], WIDTH);
        fingers.moved(1, [50.0, 560.0], &mut input);
        let _ = fingers.told();
        fingers.release(&mut input);
        assert_eq!(input.stick, [0.0; 2]);
        assert_eq!(
            fingers.told().as_deref(),
            Some(r#"{"type":"stick","stick":null}"#)
        );
        assert!(!fingers.moved(1, [50.0, 500.0], &mut input));
    }
}
