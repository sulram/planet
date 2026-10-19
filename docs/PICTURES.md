---
title: Pictures
group: Shape
summary: "A picture placed in the world: its three files, the level each distance shows, and how one that moves plays"
order: 23
---

# PICTURES

A picture is a part whose shape is an image (GLOSSARY): a quad with a
texture, lit or shining, a still or one that moves.

## Pictures

- A picture is one textured quad a draw, instanced per view, camera
  relative, with its own texture: sRGB bytes read as linear light, a mip
  chain made on the GPU as it lands, so a picture seen small or aslant does
  not shimmer. Alpha under a half is cut out, in the picture and in its
  shadow.
- Lit as a matte print, or shining with its own colour as a screen (`shine`),
  as a cell's side does; a screen's white is a sunlit white.
- Both sides, or the front alone: the back is the same quad wound the other
  way, left still left, so a poster seen from behind is not drawn and a free
  standing picture is.
- A picture not yet landed is a light grey card of its size.
- Three files a picture, the owner's client's work at placement: the
  original, a low version at most 1024 px on its long side and a thumb at
  most 128 px, both WebP. Far draws the thumb, mid the low version, near and
  in view the original: the level is picked by the quad's height on the
  screen in texels, with hysteresis, asked for over the asset seam when
  first wanted, at most four in flight, and dropped when unseen for ten
  seconds or when 96 MB of texels are resident, the longest unseen first.
- Decoding is the shell's, as fetching is: the browser's own decoder off
  the frame's thread on the web, a codec crate on desktop, so the engine
  carries none. A texture wider than the device takes is halved until it
  fits; the client asks for no more than 2048 texels a side.
- A picture that moves, a GIF, an animated WebP or an MP4, is the same
  record with its first frame as the two posters and how long it plays in
  milliseconds; its original is never fetched as pixels. In view it plays:
  the client asks the shell to open the file into a live texture the
  renderer makes (`client::Moving`), the shell decodes and writes every
  frame into it, never through the engine, and the mip chain is redrawn
  with each frame. Out of view, or a second unseen, it is closed and its
  poster shows; four play at once, the tallest on the screen first, and an
  open the shell never answers is given up after fifteen seconds.
- On the web WebCodecs' `ImageDecoder` plays a GIF, an animated WebP or an
  APNG a frame at a time, paced by each frame's duration (under 20 ms plays
  100 ms, as browsers do), and a muted looping `<video>` plays an MP4; under
  WebGL2 a frame goes through an `ImageBitmap`. On the desktop the `image`
  crate decodes a GIF or a WebP on a thread of its own; a video shows its
  poster until GStreamer arrives (ROADMAP).
- A video is heard: its reach (`reach_m`, the knob a lamp has for its
  light) says how far, 0 by its size, twelve times its height. Within its
  reach it stays open out of view, so the sound does not die when one turns
  away, and each frame the engine names every playing video with its place
  and reach (`Client::sounds`). The shell mixes them at the eye with the
  platform's audio: on the web one Web Audio listener at the origin of the
  camera's frame and a panner a source, HRTF for direction, a straight fall
  off from a metre to the reach. The browser lets sound out after the first
  press or key; a video is muted until then. The desktop plays no sound.
  A peer's voice, heard by proximity, is the next source over the same
  list and panners.
- A moving picture is placed as a screen, in the light finish, until its
  shine says otherwise.
