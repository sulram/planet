# 83. Sun shadows are read through a tent five texels wide (decided)

Logged 2026-09-30.

A point reads the sun's shadow map through a tent five texels wide, in nine
bilinear comparisons, the same in every cascade.

**Why.** In the report behind 80 the edge of a wall's shadow on its floor
came out in steps. A texel of the first cascade is 7.8 cm on the ground, and
the lookup was four comparisons half a texel apart, which is a tent three
texels wide: too narrow to hide the texel it reads through. Five wide, the
edge of a shadow is a line again, of a cube and of an avatar alike.

**What it costs.** At the place of the report, 1280x720 on the development
machine, three runs of 300 frames each way: the median frame with shadows
goes from 1.29 to 1.42 ms, and with grass on from 1.79 to 2.00 ms. Grass pays
most, having the most fragments. The edge of a shadow is 39 cm wide in the
first cascade where it was 23, and what is thinner than that casts fainter:
the shadow of a leg next to its foot.

**Not measured:** a Raspberry Pi and a phone, where a lookup costs more of
the frame than here. The shadow quality knob in ROADMAP is where a cheaper
lookup for them goes.

**Where it comes from.** Ignacio Castaño, Shadow Mapping Summary, Part 1
(2013), written for The Witness; `refs/bevy` offers the same as its fixed
blur. It is written here from the description, the weights and the places of
the taps following from the tent. The licence of bevy allows its code with
its notice, and this repository has no place for a notice until its own
licence is chosen (OPEN).

**Rejected:** a rotated kernel with noise per pixel, which needs a pass over
time to hide the noise and makes a headless picture depend on it; maps of
2048, four times the memory in every cascade since the layers of one array
share a size; a search for blockers so the edge widens with distance, which
costs more lookups again and stays a wish.

**Lives in:** RENDER.md § Sea, sky and light,
`crates/render/src/shaders/common.wgsl` (`shadow_tent`).
