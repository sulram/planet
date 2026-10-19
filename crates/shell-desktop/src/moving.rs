//! The desktop side of pictures that move: a GIF or an animated WebP
//! decoded by the `image` crate on a thread of its own, paced by each
//! frame's delay, every frame handed over a channel and written into the
//! renderer, so no frame passes through the engine. A video has no decoder
//! on the desktop yet (ROADMAP, Native video): it is refused, and its
//! poster shows.

use std::collections::HashMap;
use std::io::Cursor;
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use std::time::Duration;

use client::{Client, Moving};
use image::AnimationDecoder;
use render::{PictureFrame, Renderer};
use scene::PictureId;

/// A frame with no delay plays for this long, as in every browser.
const SHORT_FRAME: Duration = Duration::from_millis(100);
/// The shortest delay a frame is honoured at.
const LEAST_FRAME: Duration = Duration::from_millis(20);

/// What the decoding thread says: a frame, or why there are none.
type Said = Result<scene::Image, String>;

/// One picture playing: its thread's frames, and whether the engine has
/// heard its size.
struct Source {
    frames: Receiver<Said>,
    opened: bool,
}

/// The pictures playing, by the texture each writes.
#[derive(Default)]
pub struct Playing {
    sources: HashMap<PictureId, Source>,
}

impl Playing {
    /// Opens and closes what the engine asked for this frame, and writes
    /// the latest frame of every source that has one.
    pub fn tick(&mut self, client: &mut Client, renderer: &Renderer) {
        for moving in client.drain_moving() {
            match moving {
                Moving::Open { picture, path } => {
                    let (sender, frames) = sync_channel(1);
                    match crate::assets::locate(&path) {
                        Some(file) => {
                            std::thread::spawn(move || play(file, &sender));
                        }
                        None => {
                            let _ = sender.send(Err(format!("{path}: out of the desktop's reach")));
                        }
                    }
                    self.sources.insert(
                        picture,
                        Source {
                            frames,
                            opened: false,
                        },
                    );
                }
                Moving::Close(picture) => {
                    // Dropping the receiver ends the thread at its next frame.
                    self.sources.remove(&picture);
                }
            }
        }
        let mut gone = Vec::new();
        for (&picture, source) in &mut self.sources {
            let mut latest = None;
            while let Ok(said) = source.frames.try_recv() {
                match said {
                    Ok(frame) => latest = Some(frame),
                    Err(message) => {
                        client.moving_opened(picture, Err(message));
                        gone.push(picture);
                        latest = None;
                        break;
                    }
                }
            }
            let Some(frame) = latest else {
                continue;
            };
            if !source.opened {
                source.opened = true;
                client.moving_opened(picture, Ok((frame.width, frame.height)));
            }
            // The texture is made when the renderer applies the change the
            // open produced: this frame lands once that happened, and the
            // next one does when it has not yet.
            renderer.write_picture(picture, PictureFrame::Pixels(&frame));
        }
        for picture in gone {
            self.sources.remove(&picture);
        }
    }
}

/// Decodes the file frame by frame, forever, sending each when it is due,
/// until nobody listens. A still is sent once.
fn play(file: std::path::PathBuf, frames: &SyncSender<Said>) {
    let bytes = match std::fs::read(&file) {
        Ok(bytes) => bytes,
        Err(error) => {
            let _ = frames.send(Err(format!("{}: {error}", file.display())));
            return;
        }
    };
    let format = match image::guess_format(&bytes) {
        Ok(format) => format,
        Err(error) => {
            let _ = frames.send(Err(error.to_string()));
            return;
        }
    };
    loop {
        let decoded = match format {
            image::ImageFormat::Gif => image::codecs::gif::GifDecoder::new(Cursor::new(&bytes))
                .map(|decoder| decoder.into_frames()),
            image::ImageFormat::WebP => image::codecs::webp::WebPDecoder::new(Cursor::new(&bytes))
                .map(|decoder| decoder.into_frames()),
            _ => {
                let _ = frames.send(Err(format!("{format:?}: no frame decoder on the desktop")));
                return;
            }
        };
        let decoded = match decoded {
            Ok(decoded) => decoded,
            Err(error) => {
                let _ = frames.send(Err(error.to_string()));
                return;
            }
        };
        let mut sent = 0;
        for frame in decoded {
            let frame = match frame {
                Ok(frame) => frame,
                Err(error) => {
                    if sent == 0 {
                        let _ = frames.send(Err(error.to_string()));
                    }
                    return;
                }
            };
            let (numer, denom) = frame.delay().numer_denom_ms();
            let delay = Duration::from_millis(u64::from(numer / denom.max(1)));
            let delay = if delay < LEAST_FRAME {
                SHORT_FRAME
            } else {
                delay
            };
            let buffer = frame.into_buffer();
            let image = scene::Image {
                width: buffer.width(),
                height: buffer.height(),
                rgba: buffer.into_raw(),
            };
            if frames.send(Ok(image)).is_err() {
                return;
            }
            sent += 1;
            std::thread::sleep(delay);
        }
        if sent <= 1 {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::codecs::gif::{GifEncoder, Repeat};
    use image::{Delay, Frame, RgbaImage};

    /// A GIF of `frames` frames, 4 by 2, each a flat colour, 30 ms apart.
    fn gif(frames: u8) -> std::path::PathBuf {
        let mut bytes = Vec::new();
        {
            let mut encoder = GifEncoder::new(&mut bytes);
            encoder.set_repeat(Repeat::Infinite).unwrap();
            for i in 0..frames {
                let image = RgbaImage::from_pixel(4, 2, image::Rgba([i * 40, 0, 0, 255]));
                encoder
                    .encode_frame(Frame::from_parts(
                        image,
                        0,
                        0,
                        Delay::from_numer_denom_ms(30, 1),
                    ))
                    .unwrap();
            }
        }
        let path =
            std::env::temp_dir().join(format!("planet-moving-{frames}-{}.gif", std::process::id()));
        std::fs::write(&path, bytes).unwrap();
        path
    }

    #[test]
    fn a_gif_plays_in_order_and_loops_until_nobody_listens() {
        let (sender, frames) = sync_channel(1);
        let path = gif(3);
        let thread = std::thread::spawn(move || play(path, &sender));
        let mut reds = Vec::new();
        for _ in 0..7 {
            let frame = frames.recv().unwrap().unwrap();
            assert_eq!((frame.width, frame.height), (4, 2));
            reds.push(frame.rgba[0]);
        }
        assert_eq!(reds, [0, 40, 80, 0, 40, 80, 0], "in order, round again");
        drop(frames);
        thread.join().unwrap();
    }

    #[test]
    fn a_still_is_sent_once_and_the_thread_ends() {
        let (sender, frames) = sync_channel(1);
        let path = gif(1);
        let thread = std::thread::spawn(move || play(path, &sender));
        assert!(frames.recv().unwrap().is_ok());
        assert!(frames.recv().is_err(), "the thread ended after one frame");
        thread.join().unwrap();
    }

    #[test]
    fn a_file_that_is_not_there_is_refused() {
        let (sender, frames) = sync_channel(1);
        play(std::path::PathBuf::from("/nowhere/at/all.gif"), &sender);
        assert!(frames.recv().unwrap().is_err());
    }
}
