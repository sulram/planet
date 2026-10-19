---
date: 2026-10-20
lives_in:
  - PICTURES.md#pictures
---

# A picture that moves is a texture the shell plays

**Decision:** A GIF, an animated WebP or an MP4 is a picture whose record says how long it plays, with its first frame as the two posters. The engine never sees a frame: in view it asks the shell to open the file into a live texture the renderer holds, and the shell decodes with the platform's own decoders and writes every frame into it; out of view, or a second unseen, it is closed and the poster shows. Four play at once, the tallest on the screen first.

**Why:** One seam serves a loop and a clip alike: the browser decodes both frame by frame for free, memory holds one frame a source instead of every frame of a GIF, and the distance rule the stills already have becomes play and stop. GIFs of any size must play, so a moving picture ignores the texel rule that keeps a small still at its poster.

**Rejected:** Frames pre-decoded into a texture array with a clock in the engine: a second engineering for GIF alone, memory by the second of loop, and dead when video comes. Converting every GIF to WebP before upload: a megabyte of encoder in the page and no lighter engine, since a decoded frame weighs the same. Playing from the middle distance by texels: a 320 px GIF stood still at a few metres.
