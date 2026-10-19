---
title: Sound
group: Shape
summary: "What is heard: a video's sound, a sound placed in the world, mixed at the eye by the shell, and the voice to come"
order: 24
---

# SOUND

What is heard in the world, and how the shell mixes it at the eye.

## Heard

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
- A sound is a part (`Shape::Sound`): an MP3, M4A or OGG file, the three
  every browser plays, played where the part stands, looping, heard within
  its reach, sixteen metres when the reach says "by its size". It has a
  place and nothing else: no turn, no size, nothing to stand on, and nothing
  is drawn of it but its gizmo when picked; the list of what is near finds
  it. Eight play at once, the nearest first; one leaving its reach closes a
  second later.
- A video and a sound carry `muted` and `volume` (0 to 1, full when the
  record says nothing): the panel's "Sound" box silences one, and with it
  on, volume and reach show. Silenced, a video plays for the eye alone and
  is not kept open out of view; a sound is not opened. The shell holds a
  gain a source, the part's volume while the engine names it, silence
  while it does not.
- The sources are one list a frame (`Client::sounds`), a panner each in
  the shell: a peer's voice, heard by proximity, joins it next. An ambience
  for a volume, heard inside it with no fall off, waits on the volume's
  record (ROADMAP).
