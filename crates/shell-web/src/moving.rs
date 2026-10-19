//! The browser side of pictures that move: the engine says which file to
//! play into which texture (`client::Moving`), and this decodes it with the
//! browser's own decoders and writes every frame into the renderer, so no
//! frame passes through the engine. A GIF, an animated WebP or an APNG goes
//! through WebCodecs' `ImageDecoder`, a frame at a time, paced by each
//! frame's own duration; an MP4 through a `<video>` element, muted and
//! looping, a frame copied whenever its time moved. Closed, a source stops
//! and is forgotten.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

use client::Moving;
use render::PictureFrame;
use render::wgpu::{CopyExternalImageSourceInfo, ExternalImageSource, Origin2d};
use scene::PictureId;
use wasm_bindgen::prelude::*;
use web_sys::{HtmlVideoElement, ImageBitmap, VideoFrame};

use crate::web::{State, now_ms, resolve, respond};

/// A frame of a GIF that says no delay, or one under what browsers honour,
/// plays for this long: what every browser does with such a GIF.
const SHORT_FRAME_MS: f64 = 100.0;
/// The shortest delay a GIF frame is honoured at.
const LEAST_FRAME_MS: f64 = 20.0;

/// `HTMLMediaElement.readyState` with a frame to show.
const HAVE_CURRENT_DATA: u16 = 2;

#[wasm_bindgen]
extern "C" {
    /// WebCodecs' `ImageDecoder`: a still or animated image file decoded a
    /// frame at a time. Declared here, as web-sys keeps it behind an unstable
    /// flag.
    type ImageDecoder;
    #[wasm_bindgen(constructor, catch)]
    fn new(init: &JsValue) -> Result<ImageDecoder, JsValue>;
    #[wasm_bindgen(method)]
    fn decode(this: &ImageDecoder, options: &JsValue) -> js_sys::Promise;
    #[wasm_bindgen(method, getter)]
    fn completed(this: &ImageDecoder) -> js_sys::Promise;
    #[wasm_bindgen(method, getter)]
    fn tracks(this: &ImageDecoder) -> JsValue;
    #[wasm_bindgen(method)]
    fn close(this: &ImageDecoder);

    #[wasm_bindgen(js_name = createImageBitmap)]
    fn create_image_bitmap(image: &JsValue) -> js_sys::Promise;
}

/// The engine hears how an open went.
fn answer(client: &mut client::Client, picture: PictureId, size: Result<(u32, u32), String>) {
    client.moving_opened(picture, size);
}

/// Whether this browser decodes images frame by frame.
fn has_image_decoder() -> bool {
    js_sys::Reflect::has(&js_sys::global(), &JsValue::from_str("ImageDecoder")).unwrap_or(false)
}

/// What plays into one texture.
enum Source {
    /// An image file decoded a frame at a time.
    Frames {
        decoder: ImageDecoder,
        count: u32,
        /// The frame decoded next.
        index: u32,
        /// When the next frame is due, `performance.now()` ms.
        due_ms: f64,
        /// A decode is in flight: one at a time.
        busy: bool,
    },
    /// A video element, playing on its own clock.
    Video {
        element: HtmlVideoElement,
        /// Its time when a frame was last copied: a frame is copied when it moved.
        copied_at: f64,
        /// Whether the engine has heard its size.
        opened: bool,
    },
}

impl Drop for Source {
    fn drop(&mut self) {
        match self {
            Source::Frames { decoder, .. } => decoder.close(),
            Source::Video { element, .. } => {
                let _ = element.pause();
                element.remove_attribute("src").ok();
                element.load();
            }
        }
    }
}

/// The sources playing, by the texture each writes.
#[derive(Default)]
pub struct Playing {
    sources: HashMap<PictureId, Source>,
}

/// Opens and closes what the engine asked for this frame, and advances every
/// source: a frame due is decoded, a video that moved is copied.
pub fn tick(state: &Rc<RefCell<State>>) {
    let asked = state.borrow_mut().client.drain_moving();
    for moving in asked {
        match moving {
            Moving::Open { picture, path } => open(state, picture, &path),
            Moving::Close(picture) => {
                state.borrow_mut().playing.sources.remove(&picture);
            }
        }
    }
    let now = now_ms();
    let due: Vec<PictureId> = {
        let mut held = state.borrow_mut();
        let State {
            playing,
            renderer,
            client,
            ..
        } = &mut *held;
        let mut due = Vec::new();
        for (&picture, source) in &mut playing.sources {
            match source {
                Source::Frames { due_ms, busy, .. } => {
                    if !*busy && now >= *due_ms {
                        due.push(picture);
                    }
                }
                Source::Video {
                    element,
                    copied_at,
                    opened,
                } => {
                    if !*opened {
                        if let Some(error) = element.error() {
                            *opened = true;
                            answer(
                                client,
                                picture,
                                Err(format!("video error {}", error.code())),
                            );
                        } else if element.video_width() > 0 {
                            *opened = true;
                            answer(
                                client,
                                picture,
                                Ok((element.video_width(), element.video_height())),
                            );
                        }
                        continue;
                    }
                    let time = element.current_time();
                    if element.ready_state() >= HAVE_CURRENT_DATA && time != *copied_at {
                        *copied_at = time;
                        renderer.write_picture(
                            picture,
                            PictureFrame::External(external(
                                ExternalImageSource::HTMLVideoElement(element.clone()),
                            )),
                        );
                    }
                }
            }
        }
        due
    };
    for picture in due {
        decode_next(state, picture);
    }
}

fn external(source: ExternalImageSource) -> CopyExternalImageSourceInfo {
    CopyExternalImageSourceInfo {
        source,
        origin: Origin2d::ZERO,
        flip_y: false,
    }
}

/// The MIME type of a world's file, by the extension the server named it
/// with off its bytes. A video goes to a `<video>`, anything else to the
/// image decoder.
fn mime(path: &str) -> Option<&'static str> {
    match path.rsplit_once('.').map(|(_, ext)| ext)? {
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        "png" => Some("image/png"),
        "mp4" => Some("video/mp4"),
        _ => None,
    }
}

fn open(state: &Rc<RefCell<State>>, picture: PictureId, path: &str) {
    let url = {
        let held = state.borrow();
        resolve(path, &held.files)
    };
    let Some(mime) = mime(path) else {
        answer(
            &mut state.borrow_mut().client,
            picture,
            Err(format!("{path}: no decoder for it")),
        );
        return;
    };
    if mime.starts_with("video/") {
        match video(&url) {
            Ok(element) => {
                state.borrow_mut().playing.sources.insert(
                    picture,
                    Source::Video {
                        element,
                        copied_at: -1.0,
                        opened: false,
                    },
                );
            }
            Err(error) => answer(&mut state.borrow_mut().client, picture, Err(error)),
        }
        return;
    }
    if !has_image_decoder() {
        answer(
            &mut state.borrow_mut().client,
            picture,
            Err(format!(
                "{path}: this browser decodes no image frame by frame"
            )),
        );
        return;
    }
    let weak = Rc::downgrade(state);
    wasm_bindgen_futures::spawn_local(async move {
        let opened = frames(&url, mime).await;
        let Some(state) = weak.upgrade() else {
            return;
        };
        match opened {
            Ok((decoder, count)) => {
                state.borrow_mut().playing.sources.insert(
                    picture,
                    Source::Frames {
                        decoder,
                        count,
                        index: 0,
                        due_ms: 0.0,
                        busy: false,
                    },
                );
                // The first frame says the size: the engine hears it when it lands.
                decode_next(&state, picture);
            }
            Err(error) => answer(&mut state.borrow_mut().client, picture, Err(error)),
        }
    });
}

/// A video element off the page, muted so it may play with no gesture,
/// looping, loaded from the world's file.
fn video(url: &str) -> Result<HtmlVideoElement, String> {
    let document = web_sys::window()
        .and_then(|w| w.document())
        .ok_or("no document")?;
    let element: HtmlVideoElement = document
        .create_element("video")
        .map_err(|e| format!("{e:?}"))?
        .unchecked_into();
    element.set_muted(true);
    element.set_loop(true);
    element.set_autoplay(true);
    element.set_preload("auto");
    element
        .set_attribute("playsinline", "")
        .map_err(|e| format!("{e:?}"))?;
    element.set_src(url);
    // The promise rejects when the browser holds back; `autoplay` tries
    // again when it may, so the rejection is heard and let go.
    if let Ok(playing) = element.play() {
        let quiet = Closure::<dyn FnMut(JsValue)>::new(|_| ());
        let _ = playing.catch(&quiet);
        quiet.forget();
    }
    Ok(element)
}

/// An image decoder over the whole file, with its frame count once the
/// file is read through.
async fn frames(url: &str, mime: &str) -> Result<(ImageDecoder, u32), String> {
    use wasm_bindgen_futures::JsFuture;
    let describe = |error: JsValue| format!("{url}: {error:?}");
    let response = respond(url).await?;
    let data = JsFuture::from(response.array_buffer().map_err(describe)?)
        .await
        .map_err(describe)?;
    let init = js_sys::Object::new();
    js_sys::Reflect::set(&init, &"data".into(), &data).map_err(describe)?;
    js_sys::Reflect::set(&init, &"type".into(), &mime.into()).map_err(describe)?;
    let decoder = ImageDecoder::new(&init).map_err(describe)?;
    // The track, and its frame count, exist only once the track list is
    // ready; the count is final once the whole file is read through.
    let tracks = decoder.tracks();
    let ready: js_sys::Promise = js_sys::Reflect::get(&tracks, &"ready".into())
        .map_err(describe)?
        .unchecked_into();
    JsFuture::from(ready).await.map_err(describe)?;
    JsFuture::from(decoder.completed())
        .await
        .map_err(describe)?;
    let track = js_sys::Reflect::get(&tracks, &"selectedTrack".into()).map_err(describe)?;
    let count = js_sys::Reflect::get(&track, &"frameCount".into())
        .ok()
        .and_then(|count| count.as_f64())
        .unwrap_or(1.0) as u32;
    Ok((decoder, count.max(1)))
}

/// Decodes the next frame of a source and writes it when it lands; the one
/// after is due when this one has played for its duration. A source of one
/// frame writes it once and rests.
fn decode_next(state: &Rc<RefCell<State>>, picture: PictureId) {
    let (promise, webgl) = {
        let mut held = state.borrow_mut();
        let webgl = held.webgl;
        let Some(Source::Frames {
            decoder,
            index,
            busy,
            ..
        }) = held.playing.sources.get_mut(&picture)
        else {
            return;
        };
        *busy = true;
        let options = js_sys::Object::new();
        let _ = js_sys::Reflect::set(&options, &"frameIndex".into(), &JsValue::from(*index));
        (decoder.decode(&options), webgl)
    };
    let weak: Weak<RefCell<State>> = Rc::downgrade(state);
    wasm_bindgen_futures::spawn_local(async move {
        let decoded = wasm_bindgen_futures::JsFuture::from(promise).await;
        let frame: Option<VideoFrame> = decoded
            .ok()
            .and_then(|result| js_sys::Reflect::get(&result, &"image".into()).ok())
            .map(JsCast::unchecked_into);
        let Some(frame) = frame else {
            if let Some(state) = weak.upgrade() {
                let mut held = state.borrow_mut();
                held.playing.sources.remove(&picture);
                answer(
                    &mut held.client,
                    picture,
                    Err("a frame did not decode".into()),
                );
            }
            return;
        };
        let size = (frame.display_width(), frame.display_height());
        let duration_ms = frame.duration().map_or(SHORT_FRAME_MS, |us| {
            let ms = us / 1000.0;
            if ms < LEAST_FRAME_MS {
                SHORT_FRAME_MS
            } else {
                ms
            }
        });
        // WebGL2 copies no VideoFrame: a bitmap of it instead.
        let source = if webgl {
            let bitmap = wasm_bindgen_futures::JsFuture::from(create_image_bitmap(&frame)).await;
            frame.close();
            match bitmap {
                Ok(bitmap) => {
                    ExternalImageSource::ImageBitmap(bitmap.unchecked_into::<ImageBitmap>())
                }
                Err(_) => return,
            }
        } else {
            ExternalImageSource::VideoFrame(frame)
        };
        let Some(state) = weak.upgrade() else {
            close_source(&source);
            return;
        };
        let mut held = state.borrow_mut();
        let State {
            playing,
            renderer,
            client,
            ..
        } = &mut *held;
        let Some(Source::Frames {
            count,
            index,
            due_ms,
            busy,
            ..
        }) = playing.sources.get_mut(&picture)
        else {
            // Closed while decoding.
            close_source(&source);
            return;
        };
        if *index == 0 && *due_ms == 0.0 {
            answer(client, picture, Ok(size));
        }
        renderer.write_picture(picture, PictureFrame::External(external(source.clone())));
        close_source(&source);
        *busy = false;
        let now = now_ms();
        *due_ms = if *due_ms == 0.0 { now } else { *due_ms }.max(now - duration_ms) + duration_ms;
        *index = if *count > 1 {
            (*index + 1) % *count
        } else {
            // One frame: nothing more to decode, ever.
            *due_ms = f64::INFINITY;
            0
        };
    });
}

/// Frees what the browser holds of a frame once it is copied.
fn close_source(source: &ExternalImageSource) {
    match source {
        ExternalImageSource::VideoFrame(frame) => frame.close(),
        ExternalImageSource::ImageBitmap(bitmap) => bitmap.close(),
        _ => {}
    }
}
