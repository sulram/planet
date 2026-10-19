//! What is heard, mixed at the eye with Web Audio: one listener, a panner a
//! source. The engine says where each source stands this frame and how far
//! it carries (`client::Sound`); the shell owns what sounds: a playing
//! video's element today, a peer's voice stream, heard by proximity, over
//! the same panner tomorrow. Positions are given relative to the eye, which
//! sits at the origin looking down `-z`, so the world's size never meets the
//! graph's f32. The browser lets sound out only after a gesture: the context
//! resumes on the first press or key, and a video stays muted until then.

use std::collections::HashMap;

use client::{Sound, SoundSource};
use glam::DVec3;
use scene::Camera;
use wasm_bindgen::JsValue;
use web_sys::{
    AudioContext, AudioContextState, DistanceModelType, GainNode, HtmlAudioElement,
    HtmlMediaElement, MediaElementAudioSourceNode, PannerNode, PanningModelType,
};

/// Within this far of a source the sound is at its full; from there to its
/// reach it falls off in a straight line.
const FULL_M: f64 = 1.0;

/// One source in the graph: the element whose sound it is, its panner.
struct Panned {
    element: HtmlMediaElement,
    node: MediaElementAudioSourceNode,
    /// How loud: the part's volume while the engine names it, silence while
    /// it does not.
    gain: GainNode,
    panner: PannerNode,
    /// Where it stood when last heard, relative to the eye.
    at: DVec3,
    volume: f32,
}

#[derive(Default)]
pub struct Audio {
    /// Made at the first source or gesture; suspended until a gesture.
    context: Option<AudioContext>,
    sources: HashMap<SoundSource, Panned>,
}

impl Audio {
    fn context(&mut self) -> Option<&AudioContext> {
        if self.context.is_none() {
            match AudioContext::new() {
                Ok(context) => self.context = Some(context),
                Err(error) => log::warn!("no audio: {error:?}"),
            }
        }
        self.context.as_ref()
    }

    /// A gesture happened: the browser now lets sound out.
    pub fn unlock(&mut self) {
        if let Some(context) = self.context()
            && context.state() == AudioContextState::Suspended
        {
            let _ = context.resume();
        }
    }

    /// Whether sound goes out: the context runs, a gesture having happened.
    fn running(&self) -> bool {
        self.context
            .as_ref()
            .is_some_and(|context| context.state() == AudioContextState::Running)
    }

    /// A source's element enters the graph, through a panner of its own.
    /// It stays muted until sound may go out.
    pub fn attach(&mut self, source: SoundSource, element: &HtmlMediaElement) {
        let Some(context) = self.context() else {
            return;
        };
        let made = (|| -> Result<Panned, JsValue> {
            let node = context.create_media_element_source(element)?;
            let gain = context.create_gain()?;
            gain.gain().set_value(0.0);
            let panner = context.create_panner()?;
            panner.set_panning_model(PanningModelType::Hrtf);
            panner.set_distance_model(DistanceModelType::Linear);
            panner.set_ref_distance(FULL_M);
            panner.set_rolloff_factor(1.0);
            node.connect_with_audio_node(&gain)?;
            gain.connect_with_audio_node(&panner)?;
            panner.connect_with_audio_node(&context.destination())?;
            Ok(Panned {
                element: element.clone(),
                node,
                gain,
                panner,
                at: DVec3::ZERO,
                volume: 0.0,
            })
        })();
        match made {
            Ok(panned) => {
                self.sources.insert(source, panned);
            }
            Err(error) => log::warn!("{source:?}: not heard: {error:?}"),
        }
    }

    /// A source leaves the graph, and its element goes quiet.
    pub fn detach(&mut self, source: SoundSource) {
        if let Some(panned) = self.sources.remove(&source) {
            panned.element.set_muted(true);
            let _ = panned.element.pause();
            panned.node.disconnect().ok();
            panned.gain.disconnect().ok();
            panned.panner.disconnect().ok();
        }
    }

    /// A clip placed in the world starts playing, looping, off an audio
    /// element of its own in the graph: muted until sound may go out.
    pub fn open_clip(&mut self, source: SoundSource, url: &str) {
        let element = match HtmlAudioElement::new() {
            Ok(element) => element,
            Err(error) => {
                log::warn!("{source:?}: no audio element: {error:?}");
                return;
            }
        };
        element.set_loop(true);
        element.set_muted(true);
        element.set_autoplay(true);
        element.set_preload("auto");
        element.set_src(url);
        if let Ok(playing) = element.play() {
            let quiet = wasm_bindgen::closure::Closure::<dyn FnMut(JsValue)>::new(|_| ());
            let _ = playing.catch(&quiet);
            quiet.forget();
        }
        self.attach(source, &element);
    }

    /// A clip stops: its element leaves the graph and lets its file go.
    pub fn close_clip(&mut self, source: SoundSource) {
        if let Some(panned) = self.sources.get(&source) {
            panned.element.remove_attribute("src").ok();
            panned.element.load();
        }
        self.detach(source);
    }

    /// Every source put where it stands this frame, relative to the eye, as
    /// loud as its part says, and let out once the browser allows. A source
    /// the engine does not name this frame is silent: a video silenced, or
    /// out of its reach, still plays for the eye.
    pub fn tick(&mut self, camera: &Camera, sounds: &[Sound]) {
        let running = self.running();
        let eye_from_world = camera.rotation.inverse();
        for panned in self.sources.values_mut() {
            if panned.volume != 0.0 {
                panned.volume = 0.0;
                panned.gain.gain().set_value(0.0);
            }
        }
        for sound in sounds {
            let Some(panned) = self.sources.get_mut(&sound.source) else {
                continue;
            };
            if panned.volume != sound.volume {
                panned.volume = sound.volume;
                panned.gain.gain().set_value(sound.volume);
            }
            let at = eye_from_world * (sound.position - camera.position);
            if at != panned.at {
                panned.at = at;
                let [x, y, z] = at.as_vec3().to_array();
                panned.panner.position_x().set_value(x);
                panned.panner.position_y().set_value(y);
                panned.panner.position_z().set_value(z);
            }
            panned
                .panner
                .set_max_distance(sound.reach_m.max(FULL_M + 0.01));
        }
        if running {
            for panned in self.sources.values() {
                if panned.element.muted() {
                    panned.element.set_muted(false);
                }
            }
        }
    }
}
