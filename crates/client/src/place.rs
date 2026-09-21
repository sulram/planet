//! Where you are and which way you look, as a link carries it.
//!
//! The format, what each part means and why the short form is short: see
//! **Saying where you are** in docs/WORLD.md. It is one fact and it lives
//! there, because it is a fact about the world and not about this file.
//!
//! What is here and nowhere else: a pose is composed once, by the engine, and
//! handed to a front end whole. That is what keeps the web, the desktop and
//! the native panel from each writing their own version of this, and it is why
//! `Stats` carries a place *and* a pose rather than the parts.

use topology::{Column, Grid};

/// Marks the moon. Not in the code alphabet, so it can never be read as one.
const MOON_MARK: char = 'm';

/// A body, a place, and a way of looking.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Pose {
    pub on_moon: bool,
    pub column: Column,
    /// Blocks from the datum. `None` is standing on the ground.
    pub h: Option<i16>,
    /// Degrees clockwise from north. `None` at a pole, or when nobody said.
    pub bearing_deg: Option<f64>,
    /// Degrees, positive looks up. `None` when nobody said.
    pub pitch_deg: Option<f64>,
    /// Characters of code given. A short one names a box, and this is what
    /// lets arrival land in the middle of it rather than on its corner.
    pub chars: usize,
}

impl Pose {
    /// The place alone: body and code, no way of looking. What a HUD shows and
    /// what a person reads out.
    pub fn place(&self, grid: Grid) -> String {
        let code = topology::code(grid, self.column, self.h, topology::CODE_MAX);
        if self.on_moon {
            format!("{MOON_MARK}{code}")
        } else {
            code
        }
    }

    /// The whole of it, for a link.
    pub fn text(&self, grid: Grid) -> String {
        let mut out = self.place(grid);
        // Pitch without a bearing would be a half aim, so the two go together
        // or neither does.
        if let (Some(bearing), Some(pitch)) = (self.bearing_deg, self.pitch_deg) {
            out.push_str(&format!(",{bearing:.0},{pitch:.0}"));
        }
        out
    }

    /// Reads one back. The front ends hand this whatever was in the address
    /// bar, so it says what is wrong rather than guessing.
    pub fn parse(grid: Grid, text: &str) -> Result<Pose, String> {
        let text = text.trim();
        let mut parts = text.split(',');
        let head = parts.next().unwrap_or("").trim();
        let (on_moon, code) = match head.strip_prefix([MOON_MARK, 'M']) {
            Some(rest) => (true, rest),
            None => (false, head),
        };
        let found = topology::place(grid, code).map_err(|e| format!("place {code}: {e:?}"))?;

        let angle = |part: Option<&str>, name: &str| -> Result<Option<f64>, String> {
            match part.map(str::trim).filter(|text| !text.is_empty()) {
                None => Ok(None),
                Some(text) => text
                    .parse::<f64>()
                    .map(Some)
                    .map_err(|_| format!("{name} {text}: not degrees")),
            }
        };
        let bearing_deg = angle(parts.next(), "bearing")?;
        let pitch_deg = angle(parts.next(), "pitch")?;
        if parts.next().is_some() {
            return Err(format!("{text}: more than a place, a bearing and a pitch"));
        }

        Ok(Pose {
            on_moon,
            column: found.column,
            h: found.h,
            bearing_deg,
            pitch_deg,
            chars: found.chars,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use topology::Sector;

    fn grid() -> Grid {
        Grid::new(16).expect("a grid")
    }

    fn pose() -> Pose {
        Pose {
            on_moon: false,
            column: Column::new(Sector::ALL[4], 40_000, 9_001),
            h: None,
            bearing_deg: None,
            pitch_deg: None,
            chars: topology::CODE_MAX,
        }
    }

    #[test]
    fn a_pose_round_trips_whole_and_in_parts() {
        let g = grid();
        for (on_moon, h, look) in [
            (false, None, None),
            (true, None, None),
            (false, Some(40i16), None),
            (true, Some(-12), Some((180.0, -5.0))),
            (false, Some(0), Some((0.0, 0.0))),
        ] {
            let want = Pose {
                on_moon,
                h,
                bearing_deg: look.map(|(b, _)| b),
                pitch_deg: look.map(|(_, p)| p),
                ..pose()
            };
            let text = want.text(g);
            assert_eq!(Pose::parse(g, &text).expect(&text), want, "{text}");
        }
    }

    #[test]
    fn the_short_form_is_short_and_the_long_one_contains_it() {
        let g = grid();
        let ground = pose();
        assert!(!ground.text(g).contains(','), "{}", ground.text(g));
        assert!(!ground.text(g).starts_with(MOON_MARK));

        let aimed = Pose {
            on_moon: true,
            h: Some(40),
            bearing_deg: Some(180.0),
            pitch_deg: Some(-5.0),
            ..ground
        };
        // A link is the place with more said about it, never a different place.
        assert!(aimed.text(g).starts_with(&aimed.place(g)));
        assert!(aimed.place(g).starts_with(MOON_MARK));
    }

    #[test]
    fn nonsense_is_refused_with_the_reason() {
        let g = grid();
        assert!(Pose::parse(g, "9-ABC").unwrap_err().contains("Sector"));
        assert!(Pose::parse(g, "4-ABC@up").unwrap_err().contains("Height"));
        assert!(
            Pose::parse(g, "4-ABC,north")
                .unwrap_err()
                .contains("bearing")
        );
        assert!(
            Pose::parse(g, "4-ABC,0,0,0")
                .unwrap_err()
                .contains("more than")
        );
    }

    #[test]
    fn a_link_copied_by_hand_still_arrives() {
        let g = grid();
        let want = Pose {
            on_moon: true,
            h: Some(40),
            bearing_deg: Some(180.0),
            pitch_deg: Some(-5.0),
            ..pose()
        };
        let text = want.text(g);
        for variant in [text.to_uppercase(), format!("  {text} ")] {
            assert_eq!(Pose::parse(g, &variant).expect(&variant), want, "{variant}");
        }
    }
}
