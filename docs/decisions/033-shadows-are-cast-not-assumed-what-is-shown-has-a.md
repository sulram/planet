# 33. Shadows are cast, not assumed; what is shown has a floor (decided)

Logged 2026-09-21.

Two bugs from decision 32, both found by Marlus within the hour. The planet
went dark because the sphere shadow test mirrored its measure for spheres on
the far side of the sun, and the moon almost always is: every point read as
eclipsed. A sphere now shadows a point only when it lies between the point and
the sun; a body's own day side is left to the surface normal. And the moon
read as a hollow shell because collision uses the full terrain while a coarse
mesh fades small craters out and so sits higher: arriving fast, the avatar and
camera ended up under the drawn ground. The streamer now reports the footprint
the ground is drawn with at a place, and the camera and a flyer stay above the
higher of the real and the shown ground. The moon's camera floor had also been
left at zero from its smooth days. The moon gained a few wide basins so it
reads from the planet. A first try painted them as dark mare; Marlus rejected
the dalmatian look: craters are relief, never albedo. The basins are now deep
bowls with tall rims, wide enough to survive the coarsest mesh, which is the
one the planet sees.

Those basins showed a bug the small craters had hidden: walls hundreds of
metres tall, cut straight through a crater. The crater search looked at the
eight cells around the nearest lattice corner, which covers half a cell, while
a rim reaches 0.84 of one, and a basin's centre is pulled onto the surface from
wherever its cell was. A crater the search stops seeing ends in a cliff. The
search now visits every cell that can reach the sample, and a basin exists
only when its cell centre lies within 0.65 cells of the surface, so the pull is
bounded. A test walks great circles in half metre steps and fails on any jump.

The first version of that fix searched 27 cells per crater size and 64 per
basin size at every sample. Natively a patch went from 0.9 ms to 2.2 ms, which
looked affordable; in WASM it went from 1.8 ms to 8.7 ms, frames of 40 to
50 ms on the way down to the moon, which Marlus saw at once in the browser. The
whole moon holds a few dozen basins, so they are now listed once, in
`Generator::new`, and a sample only measures its distance to each; crater
cells whose box cannot reach the sample are skipped before the hash. Same
terrain to the nanometre, 0.9 ms a patch in WASM, faster than before the fix.
The lesson is a rule (CLAUDE.md, Performance): the targets are a Raspberry Pi
and a phone, cost is measured in WASM, and `bun run bench` holds the budgets.
