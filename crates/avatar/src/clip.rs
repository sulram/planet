//! Humanoid clips: Mixamo animation retargeted to the VRM humanoid at load.
//!
//! A Mixamo bone rests with some world rotation `R` (its parent with `P`),
//! while a VRM 0.x bone rests with none. A local key `q` on the Mixamo bone is
//! the same motion as `P * q * R^-1` on the VRM bone. Mixamo faces `+Z` and
//! VRM 0.x faces `-Z`, so the result is mirrored through the Y axis: negate
//! `x` and `z`. Hips motion is stored in units of hips height so it scales to
//! any avatar.

use glam::{Quat, Vec3};
use gltf::animation::util::ReadOutputs;

use crate::Error;
use crate::bones::{self, BONES, HIPS};
use crate::glb::{Glb, Hierarchy};

struct Track<T> {
    times: Vec<f32>,
    values: Vec<T>,
}

impl<T: Copy> Track<T> {
    /// The two keys around `time` and how far between them it is.
    fn around(&self, time: f32) -> (T, T, f32) {
        let next = self.times.partition_point(|&t| t <= time);
        if next == 0 {
            return (self.values[0], self.values[0], 0.0);
        }
        if next == self.times.len() {
            let last = self.values[next - 1];
            return (last, last, 0.0);
        }
        let (t0, t1) = (self.times[next - 1], self.times[next]);
        (
            self.values[next - 1],
            self.values[next],
            (time - t0) / (t1 - t0),
        )
    }
}

pub struct Clip {
    pub duration_s: f32,
    rotations: Vec<Option<Track<Quat>>>,
    hips: Option<Track<Vec3>>,
}

/// Bone rotations and hips offset for one instant. `None` leaves a bone at rest.
#[derive(Clone)]
pub struct Pose {
    pub rotations: Vec<Option<Quat>>,
    /// Hips position in units of hips height.
    pub hips: Option<Vec3>,
}

impl Clip {
    pub fn from_glb(bytes: &[u8]) -> Result<Clip, Error> {
        let glb = Glb::open(bytes)?;
        let animation = glb
            .document
            .animations()
            .next()
            .ok_or_else(|| Error::NotClip("no animation".into()))?;
        let hierarchy = Hierarchy::read(&glb.document);
        let world = hierarchy.world(&hierarchy.rest);
        let world_rotation = |node: usize| world[node].to_scale_rotation_translation().1;

        let mut rotations: Vec<Option<Track<Quat>>> = (0..BONES.len()).map(|_| None).collect();
        let mut hips = None;
        let mut duration_s = 0f32;
        for channel in animation.channels() {
            let node = channel.target().node();
            let Some(bone) = node.name().and_then(bones::from_mixamo) else {
                continue;
            };
            let reader = channel.reader(|buffer| glb.buffer(buffer));
            let (Some(times), Some(outputs)) = (reader.read_inputs(), reader.read_outputs()) else {
                continue;
            };
            let times: Vec<f32> = times.collect();
            duration_s = duration_s.max(times.last().copied().unwrap_or(0.0));
            let parent = hierarchy.parent[node.index()];
            match outputs {
                ReadOutputs::Rotations(keys) => {
                    let parent_rest = parent.map_or(Quat::IDENTITY, world_rotation);
                    let rest_inverse = world_rotation(node.index()).inverse();
                    let values = keys
                        .into_f32()
                        .map(|q| {
                            let q = parent_rest * Quat::from_array(q) * rest_inverse;
                            Quat::from_xyzw(-q.x, q.y, -q.z, q.w).normalize()
                        })
                        .collect();
                    rotations[bone] = Some(Track { times, values });
                }
                ReadOutputs::Translations(keys) if bone == HIPS => {
                    let parent_world = parent.map_or(glam::Mat4::IDENTITY, |p| world[p]);
                    let height = world[node.index()].w_axis.y;
                    let values = keys
                        .map(|t| {
                            let p = parent_world.transform_point3(Vec3::from(t)) / height;
                            Vec3::new(-p.x, p.y, -p.z)
                        })
                        .collect();
                    hips = Some(Track { times, values });
                }
                _ => {}
            }
        }
        if rotations.iter().all(Option::is_none) {
            return Err(Error::NotClip("no Mixamo bones in the animation".into()));
        }
        Ok(Clip {
            duration_s,
            rotations,
            hips,
        })
    }

    /// The pose at `time`, which wraps around the clip.
    pub fn sample(&self, time: f32) -> Pose {
        let time = if self.duration_s > 0.0 {
            time.rem_euclid(self.duration_s)
        } else {
            0.0
        };
        Pose {
            rotations: self
                .rotations
                .iter()
                .map(|track| {
                    let (a, b, t) = track.as_ref()?.around(time);
                    Some(a.slerp(b, t))
                })
                .collect(),
            hips: self.hips.as_ref().map(|track| {
                let (a, b, t) = track.around(time);
                a.lerp(b, t)
            }),
        }
    }
}

impl Pose {
    /// Every bone at rest: what an avatar wears before any clip is loaded.
    pub fn rest() -> Pose {
        Pose {
            rotations: vec![None; BONES.len()],
            hips: None,
        }
    }

    /// Moves this pose toward `other` by `amount` in `0..=1`.
    fn blend(&mut self, other: &Pose, amount: f32) {
        for (mine, theirs) in self.rotations.iter_mut().zip(&other.rotations) {
            *mine = match (*mine, *theirs) {
                (Some(a), Some(b)) => Some(a.slerp(b, amount)),
                (a, b) => a.or(b),
            };
        }
        self.hips = match (self.hips, other.hips) {
            (Some(a), Some(b)) => Some(a.lerp(b, amount)),
            (a, b) => a.or(b),
        };
    }
}

/// Plays one clip at a time and cross fades on change. Clips are named by the
/// caller's own key type, so this file knows nothing about locomotion.
pub struct Animator<K> {
    current: K,
    time_s: f32,
    /// The clip being faded out: key, its time, and the fade progress `0..1`.
    previous: Option<(K, f32, f32)>,
}

const FADE_S: f32 = 0.2;

impl<K: Copy + PartialEq> Animator<K> {
    pub fn new(start: K) -> Animator<K> {
        Animator {
            current: start,
            time_s: 0.0,
            previous: None,
        }
    }

    /// Advances time. `speed` scales playback of the current clip.
    pub fn update(&mut self, dt: f32, wanted: K, speed: f32) {
        if wanted != self.current {
            self.previous = Some((self.current, self.time_s, 0.0));
            self.current = wanted;
            self.time_s = 0.0;
        }
        self.time_s += dt * speed;
        if let Some((_, time, fade)) = &mut self.previous {
            *time += dt;
            *fade += dt / FADE_S;
        }
        if self
            .previous
            .as_ref()
            .is_some_and(|(_, _, fade)| *fade >= 1.0)
        {
            self.previous = None;
        }
    }

    /// The blended pose. `clip` resolves a key; a missing clip poses nothing.
    pub fn pose<'a>(&self, clip: impl Fn(K) -> Option<&'a Clip>) -> Option<Pose> {
        let current = clip(self.current)?.sample(self.time_s);
        let Some((key, time, fade)) = self.previous else {
            return Some(current);
        };
        let Some(previous) = clip(key) else {
            return Some(current);
        };
        let mut pose = previous.sample(time);
        pose.blend(&current, fade);
        Some(pose)
    }
}
