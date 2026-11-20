//! The camera's boom: how far behind the eye the camera hangs. Third person
//! it is metres long and the wheel draws it in and out; pulled all the way
//! in it is zero, and the camera is the eye: first person.

/// How much nearer a line of zoom brings the eye, of the boom's length.
pub(crate) const ZOOM_PER_LINE: f64 = 0.1;
/// The nearest the third person boom comes. A line of the wheel in past it
/// pulls the boom to the eye.
pub(crate) const NEAREST_M: f64 = 2.0;
const FARTHEST_M: f64 = 400.0;
/// How long the boom takes to swing to the eye or out of it, seconds. Under
/// a third of a second, so it reads as the move it is and never as the
/// camera lagging (a slow camera trick reads as a bug); long enough to show
/// which way it went, through the head or out of it.
const SWING_S: f64 = 0.25;
/// Nearer the eye than this the camera is in the head, and the own body is
/// not drawn: it would fill the view from the inside.
const INSIDE_M: f64 = 0.5;

/// The boom, where it goes and how far it has swung there.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct Boom {
    /// The third person length, metres. Kept while the boom is at the eye,
    /// so the key brings the camera back to where it was.
    third_m: f64,
    /// Whether the boom goes to the eye.
    eye: bool,
    /// How long the boom was when it set off for where it goes, metres, and
    /// how long ago, seconds.
    from_m: f64,
    swung_s: f64,
    /// Lines of the wheel pushed against the end it stands at: one whole
    /// line swings it, so a trackpad that overshoots the nearest boom does
    /// not land in the eye by accident.
    pushed: f64,
}

impl Boom {
    /// A boom already where it goes: `length_m` long, or at the eye at zero.
    pub fn at(length_m: f64) -> Boom {
        let mut boom = Boom {
            third_m: NEAREST_M,
            eye: true,
            from_m: 0.0,
            swung_s: SWING_S,
            pushed: 0.0,
        };
        boom.set(length_m);
        boom
    }

    /// Puts the boom where it goes at once: `length_m` long, or at the eye
    /// at zero, the third person length left as it was for the key.
    pub fn set(&mut self, length_m: f64) {
        self.eye = length_m <= 0.0;
        if !self.eye {
            self.third_m = length_m.clamp(NEAREST_M, FARTHEST_M);
        }
        self.from_m = self.length_m();
        self.swung_s = SWING_S;
        self.pushed = 0.0;
    }

    /// Applies lines of the wheel, positive in. Third person they draw the
    /// boom in and out; a whole line in past the nearest swings it to the
    /// eye, and a whole line out of the eye swings it back to the nearest.
    pub fn zoom(&mut self, lines: f64) {
        if lines == 0.0 {
            return;
        }
        if self.eye {
            // Nothing is nearer than the eye: a line in takes back a line out.
            self.pushed = (self.pushed - lines).max(0.0);
            if self.pushed >= 1.0 {
                self.third_m = NEAREST_M;
                self.swing(false);
            }
        } else if lines > 0.0 && self.third_m <= NEAREST_M {
            self.pushed += lines;
            if self.pushed >= 1.0 {
                self.swing(true);
            }
        } else {
            self.pushed = 0.0;
            self.third_m =
                (self.third_m * (1.0 - lines * ZOOM_PER_LINE)).clamp(NEAREST_M, FARTHEST_M);
        }
    }

    /// Swings to the eye, or out of it to the third person length it had.
    pub fn toggle(&mut self) {
        self.swing(!self.eye);
    }

    fn swing(&mut self, eye: bool) {
        self.from_m = self.length_m();
        self.swung_s = 0.0;
        self.eye = eye;
        self.pushed = 0.0;
    }

    pub fn advance(&mut self, dt: f64) {
        self.swung_s = (self.swung_s + dt).min(SWING_S);
    }

    /// How long it is this frame, metres: eased from where it set off, so
    /// the swing starts and ends gently.
    pub fn length_m(&self) -> f64 {
        let to_m = if self.eye { 0.0 } else { self.third_m };
        let t = self.swung_s / SWING_S;
        if t >= 1.0 {
            return to_m;
        }
        self.from_m + (to_m - self.from_m) * t * t * (3.0 - 2.0 * t)
    }

    /// Whether it goes to the eye: first person, even while it swings there.
    pub fn eye(&self) -> bool {
        self.eye
    }

    /// Whether the camera is in the head this frame.
    pub fn inside(&self) -> bool {
        self.length_m() < INSIDE_M
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Lives until any swing is over.
    fn swung(mut boom: Boom) -> Boom {
        boom.advance(SWING_S);
        boom
    }

    #[test]
    fn a_line_past_the_nearest_swings_to_the_eye_and_a_line_out_back() {
        let mut boom = Boom::at(2.2);
        boom.zoom(1.0);
        assert_eq!(boom.length_m(), NEAREST_M);
        assert!(!boom.eye());
        boom.zoom(1.0);
        assert!(boom.eye());
        // It sets off from where it was, and is there within the swing.
        assert_eq!(boom.length_m(), NEAREST_M);
        boom.advance(SWING_S / 2.0);
        assert!((0.0..NEAREST_M).contains(&boom.length_m()));
        let mut boom = swung(boom);
        assert_eq!(boom.length_m(), 0.0);
        assert!(boom.inside());
        // In past the eye is nothing.
        boom.zoom(5.0);
        assert!(boom.eye());
        boom.zoom(-1.0);
        assert!(!swung(boom).eye());
        assert_eq!(swung(boom).length_m(), NEAREST_M);
    }

    #[test]
    fn a_trackpad_swings_after_a_whole_line_pushed_against_the_stop() {
        let mut boom = Boom::at(NEAREST_M);
        for _ in 0..9 {
            boom.zoom(0.1);
        }
        assert!(!boom.eye());
        // Out takes it back, and in again starts over.
        boom.zoom(-0.05);
        boom.zoom(0.5);
        boom.zoom(0.4);
        assert!(!boom.eye());
        boom.zoom(0.6);
        assert!(boom.eye());
    }

    #[test]
    fn the_key_swings_to_the_eye_and_back_to_the_length_it_had() {
        let mut boom = Boom::at(40.0);
        boom.toggle();
        let boom = swung(boom);
        assert!(boom.eye() && boom.inside());
        let mut out = boom;
        out.toggle();
        assert_eq!(swung(out).length_m(), 40.0);
        // The wheel out of the eye comes back to the nearest, whatever it had.
        let mut wheeled = boom;
        wheeled.zoom(-1.0);
        assert_eq!(swung(wheeled).length_m(), NEAREST_M);
        // Turned back halfway, it swings back from where it got to.
        let mut back = boom;
        back.toggle();
        back.advance(SWING_S / 2.0);
        let halfway_m = back.length_m();
        back.toggle();
        assert_eq!(back.length_m(), halfway_m);
    }

    #[test]
    fn a_boom_set_is_there_at_once() {
        let mut boom = Boom::at(6.0);
        boom.set(0.0);
        assert!(boom.eye() && boom.length_m() == 0.0);
        boom.toggle();
        assert_eq!(swung(boom).length_m(), 6.0);
        assert_eq!(Boom::at(1.0).length_m(), NEAREST_M);
        assert_eq!(Boom::at(1e6).length_m(), FARTHEST_M);
    }
}
