//! Sounds placed in the world (`part::Shape::Sound`), speakers, as the shell
//! plays them: a clip is opened when the eye comes within its reach and closed a
//! second after it leaves, a few at once, the nearest first. Each frame the
//! open ones are named with their place and reach ([`Sound`]), for the shell
//! to mix with every other source.

use std::collections::HashMap;

use glam::DVec3;
use part::{Clip, PartId};

use crate::sound::{Sound, SoundSource};

/// How many clips play at once, the nearest first: a decoder each.
pub const CLIPS_AT_ONCE: usize = 8;

/// Frames a clip stays open out of reach: a second at sixty, so the edge
/// of its reach does not flicker it.
const UNHEARD_FRAMES: u64 = 60;

/// How far a clip is heard when its reach says "by its size": a room.
const HEARD_DEFAULT_M: f64 = 16.0;

/// Beyond its reach by this much, an open clip still counts as heard, so a
/// body on the edge does not open and close it every step.
const EDGE: f64 = 1.1;

/// A sound the shell plays: opened when the eye is within its reach, closed
/// when it stays out. The shell plays the file where [`Sound`] says, each
/// frame, through [`crate::Client::sounds`].
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Sounding {
    Open { part: PartId, path: String },
    Close(PartId),
}

#[derive(Default)]
pub struct Speakers {
    /// Open, by the frame each was last within reach.
    open: HashMap<PartId, u64>,
    /// Within reach this frame and not open: how far, which, its file,
    /// where and how far it carries.
    wanting: Vec<(f64, PartId, String, DVec3, f64)>,
    heard: Vec<Sound>,
    outbox: Vec<Sounding>,
    frame: u64,
}

impl Speakers {
    /// Opens a frame: what was heard last frame is said again by the parts.
    pub fn begin_frame(&mut self) {
        self.heard.clear();
        self.wanting.clear();
    }

    /// One sound part, where it stands and how far the part says it is
    /// heard (0 by default), against where the eye is.
    pub fn hear(&mut self, part: PartId, clip: &Clip, at: DVec3, reach_m: f32, eye: DVec3) {
        let reach = if reach_m > 0.0 {
            f64::from(reach_m)
        } else {
            HEARD_DEFAULT_M
        };
        let away = (at - eye).length();
        if let Some(last) = self.open.get_mut(&part) {
            if away <= reach * EDGE {
                *last = self.frame;
                self.heard.push(Sound {
                    source: SoundSource::Clip(part),
                    position: at,
                    reach_m: reach,
                });
            }
        } else if away <= reach {
            self.wanting.push((away, part, clip.name(), at, reach));
        }
    }

    /// Closes a frame: what stayed out of reach is closed, and what came
    /// within it is opened, the nearest first, while a place is free.
    pub fn end_frame(&mut self) {
        let frame = self.frame;
        let gone: Vec<PartId> = self
            .open
            .iter()
            .filter(|(_, last)| frame.saturating_sub(**last) > UNHEARD_FRAMES)
            .map(|(part, _)| *part)
            .collect();
        for part in gone {
            self.open.remove(&part);
            self.outbox.push(Sounding::Close(part));
        }
        self.wanting
            .sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(core::cmp::Ordering::Equal));
        for (_, part, path, at, reach) in self.wanting.drain(..) {
            if self.open.len() >= CLIPS_AT_ONCE {
                break;
            }
            self.open.insert(part, frame);
            self.outbox.push(Sounding::Open { part, path });
            self.heard.push(Sound {
                source: SoundSource::Clip(part),
                position: at,
                reach_m: reach,
            });
        }
        self.frame += 1;
    }

    /// What is heard this frame, from the parts' say to the next frame's start.
    pub fn heard(&self) -> &[Sound] {
        &self.heard
    }

    pub fn drain(&mut self) -> Vec<Sounding> {
        core::mem::take(&mut self.outbox)
    }

    /// Closes everything: the world they stood in is gone.
    pub fn clear(&mut self) {
        for (part, _) in self.open.drain() {
            self.outbox.push(Sounding::Close(part));
        }
        self.wanting.clear();
        self.heard.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use part::{ClipKind, ContentId};

    fn clip(byte: u8) -> Clip {
        Clip {
            file: ContentId([byte; 32]),
            kind: ClipKind::Mp3,
            millis: 1000,
        }
    }

    #[test]
    fn a_clip_opens_within_reach_and_closes_a_second_after_leaving() {
        let mut speakers = Speakers::default();
        let at = DVec3::new(10.0, 0.0, 0.0);
        speakers.begin_frame();
        speakers.hear(PartId(1), &clip(1), at, 0.0, DVec3::ZERO);
        speakers.end_frame();
        let opened = speakers.drain();
        assert_eq!(
            opened,
            [Sounding::Open {
                part: PartId(1),
                path: clip(1).name()
            }]
        );
        assert_eq!(speakers.heard().len(), 1, "heard from the frame it opens");
        assert!((speakers.heard()[0].reach_m - HEARD_DEFAULT_M).abs() < 1e-9);
        // Twenty metres away: out of its sixteen, closed after a second.
        let far = DVec3::new(30.0, 0.0, 0.0);
        for _ in 0..UNHEARD_FRAMES {
            speakers.begin_frame();
            speakers.hear(PartId(1), &clip(1), at, 0.0, far);
            speakers.end_frame();
        }
        assert!(speakers.drain().is_empty(), "kept to the edge");
        speakers.begin_frame();
        speakers.hear(PartId(1), &clip(1), at, 0.0, far);
        speakers.end_frame();
        assert_eq!(speakers.drain(), [Sounding::Close(PartId(1))]);
        // Its own reach says how far: a short one is not opened from ten metres.
        let mut short = Speakers::default();
        short.begin_frame();
        short.hear(PartId(1), &clip(1), at, 5.0, DVec3::ZERO);
        short.end_frame();
        assert!(short.drain().is_empty());
    }

    #[test]
    fn a_few_play_at_once_the_nearest_first() {
        let mut speakers = Speakers::default();
        speakers.begin_frame();
        for i in (0..12u8).rev() {
            let at = DVec3::new(f64::from(i) + 1.0, 0.0, 0.0);
            speakers.hear(PartId(u64::from(i)), &clip(i), at, 0.0, DVec3::ZERO);
        }
        speakers.end_frame();
        let opened: Vec<PartId> = speakers
            .drain()
            .into_iter()
            .map(|sounding| match sounding {
                Sounding::Open { part, .. } => part,
                Sounding::Close(_) => panic!("nothing to close"),
            })
            .collect();
        assert_eq!(
            opened,
            (0..CLIPS_AT_ONCE as u64).map(PartId).collect::<Vec<_>>()
        );
        speakers.clear();
        assert_eq!(speakers.drain().len(), CLIPS_AT_ONCE);
    }
}
