# DECISIONS

Append-only log. Each entry: what, why, what was rejected. Current state lives
in ARCHITECTURE.md.

Status: **decided** = Marlus said so. **proposed** = recommended in the design
conversation, not yet explicitly confirmed. Confirm or strike proposed entries
before the code depends on them.

All entries below: 2026-09-20, from the founding design conversation.

## 01. Build from scratch (decided)

The earlier idea (Oct 2025) was managed hosting for Hyperfy. It was replaced:
both Hyperfy and Cryptovoxels have readable source now, and the goal became a
world on our own terms that blends the two. Rejected: forking Hyperfy (GPL,
three.js, not our stack), building on Vircadia World (generic entity state in
Postgres, pre-1.0, solves a different problem than procedural voxel chunks).
All three remain reading material in `refs/`.

## 02. Rust client, Go server (decided)

Marlus wants to learn both. Rust + wgpu gives one client for desktop, browser
(WASM), Pi and headsets. Go fits an authoritative network server and embeds
PocketBase. One protocol schema generates both sides.

## 03. Raw wgpu and a light ECS, not Bevy (proposed)

Control matters most where Pi and Quest are tight, Marlus is already fluent in
wgpu through vybe, and Bevy breaks API every release with community-only XR.
Rejected: Bevy. Reference for the platform matrix: matthewjberger/wgpu-example.

## 04. Spherical only (decided)

A real sphere gives sun with time zones, moon, orbit, satellites and a curve
visible from afar. A flat looping map is a torus, a different topology, and was
dropped. The flat view survives as the atlas and as the simulation space.

## 05. Quad sphere with a single build band (proposed, direction accepted)

After reading Bowerbyte's "Blocky Planet": blocks aligned with gravity
everywhere beat a Cartesian ball of cubes, whose ground turns into stairs away
from the six poles. Most of the pain in that article comes from tiny planets
and digging to the core. We avoid both: building is confined to one band of one
shell, and the 8 singular corners are zoned as nature. Proposed numbers: 0.5 m
blocks, 2^16 per sector side, radius about 20.9 km. The Cartesian ball stays as
the topology for moons and micro worlds. Note: the address scheme is the save
format, so this is settled before M1.

## 06. Simulate flat, render spherical (proposed)

Inside the band the data is six flat grids. Collision, walking and editing run
in address space where blocks are unit cubes; curvature is a render function.
This also means the spherical choice is reversible: only the seam topology is
permanent, not the shape.

## 07. Hybrid voxels: smooth terrain, cubic construction (decided)

Smooth terrain (density + surface nets) is what vehicles need and what the
generator's noise already is. Cubes keep crisp Cryptovoxels architecture.
Rejected: cubes only (vehicles flip on stairs, blocky LOD from orbit), smooth
only (no straight walls). Cost accepted: two meshers, two tool sets.
References: Roblox terrain + parts, Space Engineers.

## 08. The world is a recipe; copy on first write (decided)

A planet this size has about 25 billion block columns and cannot be baked. The
database stores only modified chunks; everything else is regenerated from the
seed by every machine. Consequence: the generator version is frozen per world,
or untouched terrain would shift next to existing builds.

## 09. One generator, in Rust, run as WASM inside Go (proposed)

The server and headless agents need terrain too. Compiling the Rust generator
to WASM and running it through wazero keeps a single source of truth without
CGO. Rejected: porting the generator to Go (two implementations will diverge).

## 10. SQLite per world, PocketBase for the cold plane (decided; split proposed)

Marlus chose SQLite and PocketBase, which he already runs in production. The
split is ours: PocketBase holds accounts, worlds, volumes, roles and records;
chunks and the op log live in a separate `world.db` driven by our code, because
voxel edits are frequent, binary and tiny, and PocketBase realtime is SSE +
JSON with no Rust SDK. PocketBase is pre-1.0, so the core never imports it.

## 11. Many worlds from the start; file global, placement per world (decided; split proposed)

Cheap now, expensive later, and it gives a staging world immediately. The
refinement: the asset file is global by content hash, the placement belongs to
a world. Same for people: user global, role per world. This is the SaaS shape.

## 12. World generator runs in the client first (decided)

Generate many candidate planets and explore them walking and flying before any
database exists. This makes the offline explorer the first milestone and a
permanent preview mode.

## 13. Email identity, wallets later (decided)

Magic link in the browser, one-time code on native and headset. It removes the
entry barrier and fits a world whose authority is the server anyway. Wallets
become optional links. Chain choice is deferred because nothing binds it until
we mint something; leaning Tezos for assets (Brazilian cryptoart community),
Bitcoin as notary for world snapshots. Rejected for now: wallet-only login.

## 14. Volumes with delegated landlords, plus world admins (decided; rules proposed)

Marlus's model: he draws volumes, names a landlord for each, who may carve
smaller volumes for others; admins build anywhere. Proposed rules: integer
address boxes inside one sector, child inside parent, siblings never overlap,
power flows down only, revoking never deletes work, admins are logged too.

## 15. Media like Hyperfy, then uploads (decided; details proposed)

Public URLs on a CDN are allowed. Because building is restricted to trusted
people, uploads (image, video, GLB) are allowed too, stored on the Hetzner
Object Storage already attached to PocketBase. Later business: sell data
packages, which is only a quota number. Proposed details: thumbnails made by
the owner's client, heavy media served straight from the bucket, proximity
loading with a decoder budget. Known cost of external URLs: CORS and visitor
IP exposure.

## 16. Destruction: instigator decides, others watch particles (decided; modes proposed)

Marlus's model. Refinement: removed voxels are world state and go through the
server as a permission-checked op; debris is cosmetic and simulated locally
from a shared seed. Proposed per-volume modes: protected, ephemeral, permanent.
Rejected: structural collapse physics.

## 17. Web UI in Svelte + Bun; minimal UI elsewhere (decided)

Builder and player modes are full in the browser. Pi, headsets and the native
executable get a simplified UI. Consequence: tool logic lives in Rust behind a
command/event seam so every client shares it.

## 18. Repo conventions (decided)

CLAUDE.md at the root, AGENTS.md as a symlink so Claude, Codex and Kimi share
one guide; docs/ in the style of vybe and Plataforma ITS; scripts and packages
separated; Conventional Commits with Semantic Release; no AI co-author
trailers; `refs/` gitignored for reference checkouts.

## 19. Gravity as fields, after Super Mario Galaxy (proposed)

Gravity is decoupled from geometry: invisible volumes with a direction rule,
range and priority. It makes the quad sphere, moons, inverted rooms and planets
inside a voxel the same mechanism.

## 20. NVIDIA neural rendering is deferred (proposed)

DLSS 5 is RTX 50 only and has no wgpu path, so it cannot reach Pi, Quest or the
browser. The renderer writes depth and motion vectors from day one to keep the
door open. The realistic route is a cloud "cinema mode" streamed over WebRTC.

Entries below: 2026-09-20, from the first vertical.

## 21. Versions count from 0.0.1; `dev` publishes prereleases (decided)

Marlus asked for a `dev` branch that counts `0.0.1-dev.N` before anything
reaches `main`. Semantic Release does it with a prerelease branch and the tag
`v0.0.0` as anchor. Before 1.0.0 the rules are: BREAKING CHANGE bumps minor,
feat, fix and perf bump patch, so 1.0.0 is a deliberate act. At 1.0.0 the
custom rules are deleted and the defaults apply. Rejected: the default rules
from day one (the first feat would publish 1.0.0).

## 22. The first vertical cuts through M0, M1 and M2 (decided)

Marlus chose to build one thin slice end to end instead of finishing M1 first:
design system, login, backoffice, the engine with a third person avatar on a
generated planet, and "create world" writing the recipe. The M1 discipline list
in CLAUDE.md was relaxed for exactly these items. Everything else in it stands.

## 23. Web app follows Plataforma ITS; PocketBase stays a Go library (decided)

SvelteKit on adapter-node, a PocketBase client per request, the session in an
httpOnly cookie, OTP as magic link, the design system in `$lib/ds` with a live
catalogue, flat i18n keys, scripts in Bun: all as in Plataforma ITS, which
Marlus runs in production. The one difference is decision 10: PocketBase is
embedded in our Go binary, not run as its own binary, because the hot plane
needs its permission cache invalidated by hooks in the same process. Accounts
are created by a server hook on the first code request, so the web app holds
no superuser credentials. Rejected: a static SPA served by Go (the token would
live in storage readable by scripts).

## 24. Operator, a global flag (decided)

The backoffice needs a gate, and roles are per world. An operator is a user
with `users.operator = true`: the person who runs the instance. It is not the
PocketBase superuser (panel login) and not the per world admin. Guarded by an
API rule with `:changed`, so only operators change it, and user creation over
the API is closed because the default public create would allow self grant.

## 25. Tangent warp, computed seams (proposed)

Settles the OPEN question on the pre-distortion mapping: `tan(s * pi / 4)`.
It is one line, exactly invertible with `atan`, deterministic through `libm`,
and bounds the block edge between 0.35 m and 0.5 m. Seams are derived from
integer cube geometry rather than a hand written table, so the mandatory
property tests check geometry, not typing. Rejected for now: Everitt and other
equal area warps (more math for a gain that does not matter at this size).

## 26. Far terrain is a heightfield quadtree; the avatar is procedural (decided)

Walking a planet needs ground to orbit LOD before it needs editable voxels, so
the first terrain is heightfield patches from the generator. Surface nets
chunks replace only the deepest levels later, and the structure stays. The
avatar is a figure of boxes until the avatar format question in OPEN is
settled. Patches are built on the main thread under a per frame budget, which
is the one approach that runs the same on native and WASM; worker threads are
an optimisation behind the same queue.

## 27. PocketBase 0.40 needs Go 1.27 (decided)

The 0.40 line declares `go 1.27`. We pin 0.40.4 exactly and let the Go
toolchain fetch itself. The recipe freeze is a validate hook, not an API rule,
because rules skip superusers and Go side saves.

## 28. Avatars are VRM, animated by shared humanoid clips (decided)

Marlus supplied a first set of CC0 VRM avatars and asked for Hyperfy's basic
locomotion: idle, walk, jog, jump, fall, float. Clips are authored once on a
Mixamo rig and retargeted at load to the VRM humanoid, so every avatar shares
them. An anonymous visitor gets a random avatar from the default set, kept in a
cookie. Changing avatar happens in the world, not in a menu: a changer entity
you walk through opens a dialog, and a dropzone lets a builder place a VRM on
the map (both on the ROADMAP). The box figure stays as the fallback while
assets load. This settles the OPEN avatar format question for VRM; custom voxel
avatars stay a wish. Hyperfy is read for architecture only. Its clip files (GPL) are
committed as a temporary default set by Marlus's call, to be replaced (see OPEN).

## 29. Generator v2; the sea is a surface of its own (decided)

Marlus found v1's mountains needle like and wanted a sea to swim and dive in.
v1 stays frozen for the worlds that use it; new worlds get v2: massifs from a
slope damped fractal sum (after Quilez) whose first octaves stay smooth, a sea
floor with shelf, plain and seamounts, and sampling filtered by the footprint
of the mesh that asks, so coarse patches neither alias nor pop. Collision and
anything saved sample at full detail. The terrain mesh now carries the real
ground, and each patch that dips under sea level carries a water surface too:
blended, double sided, coloured by per channel absorption, Fresnel to the sky,
Snell's window from below. Underwater is a medium, not a fog colour: red dies
first. Rejected: tinting the terrain mesh blue (v1's look, not penetrable), and
a single planet sized water sphere (no depth per vertex, no shoreline). Next
step, on the ROADMAP: refraction from the scene depth and colour.

## 30. Terrain look: procedural detail anchored to the planet (proposed)

From docs/TERRAIN_RENDER_BRIEF.md, deliveries 1 and 2, without texture files
yet: gloss is explicit in the vertex contract, rock is exposed per pixel from
the slope (the same at every LOD), and three scales of value noise shade and
bump the ground. The noise lattice repeats every 1024 m and each patch passes
its origin wrapped to that period in f64, so detail is fixed to the planet and
no large f32 position is ever built. Light gained a sky and ground ambient, a
filmic tone curve, starlight and moonlight. Textures with recorded provenance,
shadows and grass remain on the ROADMAP.

## 31. The moon is a place, and the first gravity field (decided)

Marlus wanted to fly to the moon, find it bigger, be pulled upright by it and
walk on it with a higher jump. The moon is now a sphere with a real position
(an orbit on rails, a function of the clock) and a real radius, drawn by a ray
and sphere test in the sky pass, so it grows as you approach and shows correct
phases from anywhere. The controller knows which body holds the avatar: inside
the moon's field (a sphere with a range, as decision 19 describes) the position
is stored relative to the moon, so the avatar rides its orbit for free, down is
the moon's centre, gravity is a fifth, and the shown up eases round instead of
snapping. Leaving takes a little more height than entering (hysteresis). The
moon is a smooth ball for now: terrain on it needs the ball topology, which
stays on the ROADMAP. The atmosphere shell doubled to 5 km with a thinner
density, by Marlus's eye. Rejected: keeping the moon as a painted disc with a
fake "arrival" trigger.

## 32. The moon is terrain; flight owns its frame; gravity is continuous (decided)

Testing decision 31, Marlus found that the far side of the moon hid the stars
but not the sun or the planet, that the moon had no craters, and that the
approach jolted. All three came from shortcuts. The painted sphere lived in
the sky pass, behind everything with depth. The moon is now a second body on
the same terrain quadtree: `Body::Planet` and `Body::Moon` differ in radius,
depth and having a sea, patches are built around their body's centre, and the
renderer adds the centre of the moving body each frame. A crater generator
(stacked cell grids of bowls with rims, footprint filtered like the planet)
joins generator v2, guarded by its own golden hash. Depth does the occlusion;
sunlight is what neither sphere shadows, which gives night and eclipses alike.

The jolts had two causes: the avatar only started riding the moon 6 km out,
while it sweeps past at 730 m/s, and down flipped in one frame. Marlus asked
that flight predominate. So three things are separate now. The frame of
reference (sphere of influence, 4 moon radii, with hysteresis) changes far
away where nothing shows. Gravity turns continuously from one body to the
other with distance. And the avatar has a frame of its own, `frame_up`, that
turns toward gravity by rotation, carrying view and facing with it: quickly on
foot, in flight only near a surface. Look at the planet from the moon and fly:
you go there, and nothing turns you on the way. Rejected: easing only the
drawn body while the controls snapped (what decision 31 shipped). The
atmosphere settled at 3.6 km. The avatar's `Body` type became `Figure`:
in the GLOSSARY a body is celestial.

## 33. Shadows are cast, not assumed; what is shown has a floor (decided)

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

## 34. Sun shadows in cascades, casters apart from the view (decided)

Sun shadows are three cascades around the eye (40 m, 400 m, 4 km), not one map
near the player. The first version had two, ending at 640 m: a mountain on the
horizon neither cast nor received, and at a low sun the far field stayed lit
while the near field was dark. The third cascade costs one more pass of coarse
patches; on the development machine the three together add 0.4 ms a frame
(native 1280x720, 0.95 to 1.37 ms).
Casters are chosen by the client from built ancestors, apart from the drawn
list, so an offscreen hill still shades the view and no generation is asked
for. Rejected: a horizon angle baked per vertex (a generator cost per sample,
and it cannot see avatars or builds).
Not done: a quality choice in the shells, which the Pi will need.

## 35. Grass in reach tiers (decided)

Grass went through a flat candidate grid first: one tuft per block, thinned by
the trailing zeros of its address, with a fixed instance budget that dropped
whole patches. It read as sparse, ended 50 m out and showed a lattice far away.
Now tier `k` holds one jittered tuft per `2^k` half blocks and reaches
`20 m * 2^k`. Spacing and reach double together, so the count per tier is
constant (about 20 000 in the full disc) and screen density stays level out
to 640 m; far tufts grow in the shader to stand for the meadow between them.
A patch `j` levels short of the finest is only drawn from about `27 * 2^j` m
(it splits otherwise), so it carries tiers from `j + 1` up: 1365 tufts, 55 KB.
That bound ties `GRASS_TIER_0_REACH_M` to `SPLIT_DISTANCE`; raise one and the
other must follow. Measured: WASM worst frame of the descent 8.41 to 9.03 ms
(budget 12); native 1280x720, 1.37 to 1.81 ms a frame over shadows alone.

## 36. One scene target, a chain of stages, one tone map (decided)

Every scene shader used to tone map and encode its own output, straight into
the presented target. Nothing could come after: bloom needs light above white,
clouds need the scene's depth, and water blended over already curved colour.
The world is now drawn into an HDR target in linear light, its depth kept, and
a compositor runs full screen stages over it, ending in `output` (exposure,
filmic curve, encoding). An effect is a WGSL fragment entry and a line in the
chain, in the manner of a post process stack. Cost: two half float targets per
view (15 MB at 1280x720) and one full screen pass. Rejected: tone mapping per
shader with a separate bright pass for bloom (two truths about exposure), and
a generic pass graph (no second consumer yet).

## 37. Clouds are a marched shell of weather, at half size (decided)

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

## 38. Settings are data in the frame; egui on desktop, painted by us (decided)

The renderer held its own `Effects`, set by a method only the headless shot
called. A UI needs them on the seam, so they moved: `scene::Effects` is plain
data, a UI sends `set_effects`, the client clamps and answers
`effects_changed`, and every `Frame` carries what is in force. The renderer
keeps no setting. Weather had been a function of the clock alone; with the wind
a knob that would jump the sky, so the renderer now advances it frame by frame
in f64, and starts from the clock alone on a first frame or a set clock, which
keeps a headless shot a function of its clock.
Desktop UI is egui (closes the question in OPEN), in its own crate behind the
seam, as CLAUDE.md asks of integrations. `egui-wgpu` pins wgpu 29 and we ride
30, as vybe found: egui and egui-winit come from crates and the painter is ours,
some 300 lines, since egui's contract is textured triangles and scissors.
Rejected: holding wgpu back for a UI crate. On the web the choice is kept in
`localStorage`, not the account: a phone and a desktop want different answers.

## 39. Bloom is a pyramid; the tone map is a choice (decided)

Bloom is the first effect the compositor was built for. Light over a threshold
is halved down five levels and summed back up (the Call of Duty: Advanced
Warfare filter): thirteen taps down with a brightness weighted first halving,
so one blazing pixel cannot flicker the glow, a tent up. Wide glow, small
kernels, all at half size or less. Rejected: a separable Gaussian at full size
(costlier for a narrower glow). It runs after the clouds.
The tone map became a setting rather than a verdict: ACES stays the default
for its contrast, AgX is there because ACES skews bright hues (a sunset goes
yellow), Khronos neutral keeps authored colours for builders judging a
material, Reinhard and linear are references. One curve would have been a
taste imposed; five cost a switch in one shader.

## 40. The sea reads the scene behind it (decided)

The sea was a blended surface with one alpha: it could not tint the floor per
colour, bend it, or show anything from below but a flat ceiling, and under it
the whole world was fogged by the full distance, so a hill on the shore a
metre of water away vanished. It is now drawn after the opaque world and sky,
in its own pass, reading a copy of the picture and the depth: refraction and
per colour absorption over the water really crossed from above; from below,
the world above bent into Snell's window. Things past the surface are drawn
with air alone and the surface lays the water over them, so water is counted
once. Cost: one full target copy a frame and a pass without depth testing
(the shader discards behind nearer scenery). Rejected: a second render of the
scene for refraction (twice the terrain). From below, the physical bend was tried and dropped: it
folds the whole horizon into a cone overhead, so the shore hung in the sky,
the look toward it was a dark mirror, and a swimmer in front of the surface
left a ghost where their pixels were refused. The world above is now seen
straight through, rippled by the swell, a mirror only at a glancing look: a
swimmer sees the shore where it is. Only what lies past the surface along its
own ray is shown, read one texel at a time so no edge blends in.
