# 37. Clouds are a marched shell of weather, at half size (decided)

Logged 2026-09-21.

Clouds had to hold from the ground, from inside and from orbit, and shade the
land. A textured dome or billboards fail the second and third. They are a
density field in a shell (1100 to 3000 m): a weather noise says where clouds
may stand, a Perlin-Worley body is cut by a threshold that rises with height,
so each heap narrows into a dome of its own instead of meeting a ceiling, and
Worley detail carves the edges. All of it reads one tiling 64^3 texture drawn
once on the GPU (slices side by side in a 2D atlas, then copied into the
volume: wgpu 30 does not pass `depth_slice` to a browser, so a 3D slice cannot
be a render target there): procedural noise per march sample cost several
times more, and a CPU bake would stall a browser's first frame.
The same field is sampled once along the sun by every lit surface: cloud
shadows move over the land for the price of three texture reads.
At full size the march cost about 4 ms at 1280x720 on the development machine,
which no browser tab or Pi would survive. It runs at half size from the
farthest of each four depths, and a full size stage lays it over the scene and
cuts it where terrain is nearer than the layer: about 0.4 ms. Weather moves by
rotation about the planet's axis (a translation would push clouds through the
ground somewhere). Clouds also reshape where they stand: the noise rises
through the layer along the local up, the third dimension spent as time. A
sideways slide was tried first and read as more travel, not change. No repeat
of the noise lines up with a local up, so the rise cannot wrap unseen: it
swings over 64 repeats instead, continuous for ever. Both are computed in f64
on the CPU, so a long clock costs no precision. Rejected for now: temporal
reprojection (needs motion vectors, ROADMAP).
