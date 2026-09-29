# 84. Shadow quality is a knob with levels, and off is the lowest (decided)

Logged 2026-09-30.

How good the sun's shadows are is a setting with levels, beside the other
knobs of the compositor, and off is the lowest of them. A level sets how many
cascades there are, how wide a texel of each is, and how the map is read.
One knob, never one per parameter.

**Why.** Marlus asked for it after 83 and the fourth cascade in OPEN. Each
buys a better shadow with frame time, measured on the development machine
alone, and the targets are a Raspberry Pi, a phone and a browser tab, where
the same lookups cost several times more. A switch gives every screen one
answer: what the Pi can pay, or a picture the Pi cannot run. A knob lets the
cost be chosen where it is paid, and a better shadow lands as a level rather
than as a verdict for every screen.

**Not decided here:** the levels, what each sets, and which is the default
where. They come with the code and its measures, on the Pi as well.

**Rejected:** on and off alone, which `scene::Effects::shadows` is today; a
knob for each parameter (cascades, texel, lookup), which hands a person the
renderer's internals to tune.

**Lives in:** ROADMAP § Wishes (the compositor as knobs), then
`scene::Effects` and `render::shadow`.
