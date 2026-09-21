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

## 41. The shore is painted per pixel, by height (decided)

Far coasts showed teeth, worse with distance. The ground was already smooth;
the color was not: sand was a class decided per vertex and interpolated, so
its edge ran along the diagonals of the mesh, a kilometre a tooth at the
coarsest level, and a band eight metres tall was caught by a vertex only by
luck. Sand and sea floor are now decided in the terrain shader from the
height of the pixel over the sea, as rock already is from the slope: the line
is the contour of the ground, smooth at every LOD, wandered by the anchored
detail up close and softened by the pixel's own reach from afar. It costs no
vertex data. A vertex under the shore carries the cover of the highest land
beside it, so no beach bleeds uphill across a coarse triangle. The generator
is untouched: its materials are hashed and frozen. The shader holds the shore
heights of generator v2 (2 m, -6 m); a v1 world gets the same beach. Rejected:
a blur after the fact (softens everything, leaves the teeth their shape);
continuous cover fields in the vertex for snow and forest too (the same cure,
a new vertex contract: ROADMAP).

## 42. The moon shines over the threshold; nothing adapts by itself (decided)

The moon would not glow at night without bloom and exposure retuned by hand,
which then blazed by day: the moon is a sunlit rock worth a quarter of a
unit, far under a threshold that spares a meadow. To an eye used to the dark
it blazes; nothing here adapts, so the moon in the night sky is drawn six
times brighter instead, over the threshold, and blooms as anything bright
does: the path a lamp or a screen will take. Not the moon one stands on, nor
the moon by day: the rule that drowns the stars in daylight gates it. The
bloom threshold is of exposed light, as in a lens. Tried and dropped, by
Marlus's eye: a night gain on the exposure (the moon glowed and the night
became a dim day); a night gain on the glare alone, known from whether the
sky over the camera is lit and the sun reaches it (every star bloomed, and
rising out of the planet's shadow cut the glow off at once while the view had
not changed); a halo painted in the sky around the disc (not bloom: it did
not look like light spilling). Rejected: measuring the picture as Unreal does
(over the night side it is dark, the glare rises, and a sunlit avatar blazes
all the same; it pumps, costs passes, and ties a headless shot to the frames
before it); selective bloom through the scene target's alpha (a mask every
stage must carry, for one body, where brightness already says it).

## 43. The sea's waves are band limited, and what is lost becomes roughness (decided)

The swell was three fixed scales (4 m, 1 m, 0.25 m) faded out past 400 m,
because from further each one fell inside a pixel and shimmered. So the sea
went flat from the sky, the sun's mirror became a perfect disc, and from the
sea floor the surface stopped swimming a few metres down, where the same fade
killed the ripple. The waves are now six octaves, 0.25 m to 256 m, each kept
only while a pixel can show it: the rule the generator already uses for
terrain (29), asked per pixel from the derivative of the position instead of
from a mesh spacing. There is always an octave in reach, so the sea never
shimmers and never goes flat. What a pixel can no longer show is not
discarded: the variance of those slopes widens the sun's specular lobe and
dims it in proportion, so the glint spreads into the road of light a real sea
has (the reasoning of LEAN and Toksvig normal filtering, on an analytic
spectrum instead of a mip chain). Refraction through the surface became an
angle rather than a displacement capped at 6 m, so what is far swims as much
as what is near. Value noise gained an analytic slope (`value_noise_slope`),
one lookup where a finite difference took three: with the dead octaves
skipped, a water pixel now costs about what it did. Rejected: stretching the
old fade further (the fine scales shimmer again); normal maps with mip chains
(texture files, and the look is procedural first); an FFT ocean or Gerstner
waves in the vertex (a system of their own, and the water mesh is coarse
where the problem is).

## 44. A world's shape is a source: plates, or a baked field (decided)

A world could only ever be its seed. People want to walk the Earth, and the
answer is not a second generator: it is that the body of the generator should
not know where its shape came from. Generator v3 reads two broad numbers,
`land` and `ranges`, plus a `lift`, and everything under them, the coast
easing, the shelf and abyssal profile, the eroded massifs, the materials, is
the tuning v2 already earned. `params.source` picks who answers. `generated`
lays about eighteen tectonic plates over the sphere, each with a Euler pole,
and reads the boundary between the two nearest at every sample: convergence
raises a cordillera or digs a trench with an island arc beside it, divergence
opens an oceanic ridge or a continental rift. That is why v2's ranges came out
as patches and these come out as arcs; a range is the edge of a plate, not a
blob of noise. `field` reads a cube map baked from a real body, one face per
sector, and the seed still owns everything below a texel, so one field is a
family of worlds that share a coastline rather than one world.

The field is not used as a height, and this is the whole of the scale problem.
Our planet is 1/305 of Earth: honest elevations make a billiard ball (Everest
becomes 29 m), and the exaggeration that fixes that, about 48x, turns every 1
degree slope into 48 degrees. What survives the change of scale is the shape.
Zero maps to zero, so the coastline lands exactly where the source has one,
and the relief inside a belt is the generator's own. The first attempt fed
elevation and ruggedness straight in at texel resolution and grew a forest of
needles: the broad channels are now read at their own footprints (250 m for
lift, 1000 m for ranges) while the coast stays at the mesh's.

Two channels per texel: elevation as `i16`, and ruggedness, the spread of
elevation inside a finest texel, as a byte of 16 m steps. Ruggedness is what
tells a cordillera from a plateau of the same height, and coarse levels average
it rather than accumulate it, so a silhouette cannot change because the camera
moved away. Levels blend by footprint exactly as `band` fades an octave. Each
face carries a one texel gutter filled from the ground that continues past the
edge, so a seam is an ordinary texel and no seam logic runs per sample. In
WASM the field path costs 0.90 ms per patch against a budget of 1.5, and its
descent is cheaper than the generated one (6.99 ms against 9.81): fetches beat
octaves.

The default field is ETOPO 2022 (NOAA NCEI), a work of the United States
government and so public domain, baked by `bun run field` to 1024 texels per
face side, 32 m of planet and 9.8 km of Earth per texel, 25 MB with its mip
pyramid. A recipe names a field by the content id the bake writes into its
header, and the generator refuses ground that is not the ground the recipe
names: a world shaped by other ground is a different world, and its stored
chunks would no longer line up. `Generator::new` refuses a recipe that needs a
field, so nothing can quietly fall back to another planet.

Rejected: an API call at world creation (the endpoint moves, the grid is
revised, and the same recipe silently generates another planet under chunks
already saved; a field is downloaded once, baked once and frozen by its hash);
equirectangular storage (a seam, a pole singularity, and trigonometry per
sample, against a cube map that is already our sectors); a hydraulic erosion
bake for the generated source (a week of work and a `Generator::new` that is
no longer instant, and plates alone carry most of the reading; it stays a
ROADMAP wish); shipping the field in the repo (25 MB, and `bun run field`
rebuilds it, so it is gitignored like the rest of the bake).

## 45. The sea has a shape of its own, and the knobs are at the address (decided)

A field world drowned nothing. The Channel is 40 m of a body whose range is
11 km, so a depth taken in proportion put it under 0.7 m of water, and Britain
grew a land bridge to France; the North Sea, the Baltic, the Persian Gulf and
the Yellow Sea went the same way. The land had been given an exaggeration and
the sea had not. `sea_curve` is the exponent the depth follows down from the
shore: `1` is the old proportion, `0.45` is the default and puts the Channel
42 m down while the abyssal plain still arrives at all of `ocean_depth_m`.
`sea_level_m` moves the coastline on the source body, which is what
`sea_share` does for a generated world and could not do for a field: raise it
to drown the lowlands, lower it to walk out onto the shelves, as the Channel
was walked in the ice age.

`tests/earth.rs` now asserts both ends, because they pull against each other:
five shallow seas have to be deep enough to be seas, and the South Pacific has
to still reach the floor the recipe names. The first fails at 0.7 m on the old
curve, which is what makes it a test and not a restatement.

The knobs then had to be reachable. Every param of a recipe that changes what
a planet looks like is a slider in `/play` and a parameter of its address, so a
planet stays shareable and "create world" saves the one that was previewed.
`KNOBS` in `$lib/world.ts` is the single table: range, step, default, and which
shapes a knob means anything for, because the URL, the sliders, the form and
the clamp on the server all have to agree and agreeing is cheaper than
checking. The server clamps again on create: the form is a suggestion, the
range is the rule. A knob left at its default is absent from both the address
and the recipe, so a recipe carries only what someone chose. `Slider` gained
`onchange`, which fires when a drag ends, so the picture answers per frame
while the address is rewritten once.

**The planet stays at `2^16`.** Growing it was tried, measured and reverted.
Each bit doubles the radius, and at `2^19` (167 km, 1049 km around) Chile is
4.7 km wide instead of 590 m and Thailand is a place rather than a ridge. But
the same change takes the vertical exaggeration from 48 to 6 and the planet
reads flat, and the atmosphere, tuned as a uniform shell for a 20.9 km body,
turned every distance white until its density and height were re-paired. The
size and the drama are one dial, not two, and `relief_m` can buy the drama
back (2500 at `2^18` gives 22 times and four times the ground). The finding is
in OPEN.md with its numbers, against the question that was already there.

Rejected: lifting shallow water by clamping a minimum depth (a step at the
coast instead of a slope); baking a second field of "sea floor" at a different
scale (a second read path, and the curve is one `pow`); knobs as a settings
panel instead of the address (a planet is a recipe, and a recipe you cannot
paste to someone is not one).

## 46. The desktop is a whole player, and the cold plane moves to the seam (proposed)

The desktop build can render any planet and walk it, and can do nothing else:
no sign in, no list of worlds, no way into one. It is not a client, it is a
viewer with command line flags. That has to change, and changing it well is
not a question about egui.

**What it changes.** CLAUDE.md says "Svelte (web) and the *minimal* native UI
sit over one command/event seam". This entry proposes dropping the word
minimal for everything a player does, and keeping it for everything an
operator does: sign in with a one time code, list worlds, enter one, preview a
planet with its knobs and create a world from it, and the settings panel that
already exists. The backoffice stays on the web. It is table heavy, it is
rare, it is for the few people who run an instance, and it is the one place
egui would cost far more than it returns.

**Why egui is not the hard part.** Its contract is textured triangles and
scissors, our painter already serves all of it (38), and the design system it
would have to match is monospace, flat, black and white. A list, a code field,
sliders and a picker are what egui is good at. The hard part is elsewhere.

**Where the hard part is.** Today the cold plane lives in the web app's server
routes: SvelteKit calls PocketBase with its SDK, validates, clamps and writes.
None of that is reachable from Rust, which is the real reason the desktop can
do nothing. So the calls move behind the same command and event seam the
engine already sits on: `list_worlds`, `create_world`, `enter_world` as
commands, `worlds_listed` and the existing `rejected` as events, with the
clamping that `+page.server.ts` does now living in the client crate where both
front ends reach it. Then the invariant is finally true rather than aspired
to: one seam, two thin front ends. The engine has worked this way since the
start; nothing else has.

**What it costs, named.** The web app forbids a user visible literal string
and keeps `en.ts` as the source of truth. A Rust front end needs the same
strings, and two sources of truth for them would rot within a month. They are
generated from `en.ts` into a Rust module by a script, the way `proto/` will
generate the wire types for three languages. The second cost is the one to
watch and not to pay twice: every screen a player sees now exists in Svelte
and in egui, so a screen that belongs to neither should be built for neither.

**What this is not.** Not a webview inside the desktop app: it drags a browser
into a binary that also has to run on a Raspberry Pi, a Quest and a Pico
(M6), where there is no webview and a controller instead of a mouse. A native
UI that stays a panel survives that; a native UI that is a whole application
does not. That is the argument for holding the line at player screens.

**How a world is opened, and it is a URL rather than an id.** An id alone
assumes there is one instance, and this is self hostable free software, so
which instance is never optional. The web already addresses a world at
`/w/<id>`, so the desktop takes the same string a person copies out of the
browser: `bun run desktop <url>`, with a bare id meaning the instance it is
configured for. One address works in both, pasting a link becomes the natural
gesture, and it is the same scheme that will carry a place inside a world
(ROADMAP, M1) rather than a second one. It also gives `bun run shot --world`,
which the backoffice and CI both want.

That flag is not a cheap one, and saying so is the point: a recipe lives on the
server, so `--world` is already the thin end of this entry rather than a
sibling of `--seed`. It needs the seam, an HTTP client, and a local cache of
recipes, which permanent offline mode wants anyway.

**An instance's main world is not a desktop concept.** What the front door
opens is a property of the instance, read by the web and the desktop alike, so
it belongs in the cold plane beside `worlds` and not in a native flag.

Rejected: the desktop as a viewer with flags (what we have, and the complaint
that opened this); the desktop as a full peer including the backoffice (two
admin UIs, forever, for the few); a webview shell (no Pi, no headset); two
string tables (rot); a world id with no instance (one host, forever).


## 47. Collision reads the field, and a cave is ground (decided)

A heightfield has one ground per column, so a walker on one stands on the roof
of every cave it crosses. With the deepest quadtree level meshed from the
density (44, and the entry before this one in ROADMAP), you could see a cave
mouth and an arch and then walk over both. This entry says what holds a body
up now.

**A footing.** What a body stands on is the top of the solid at or under its
feet, and what is over its head is the bottom of the solid above them. The
surface out in the open, the cave's own floor and roof inside one. It is read
from a column of the density field, probed down from the feet, and refined by
bisection to the millimetre.

**Why not a collision mesh.** ROADMAP asked for a representation of its own,
coarser than the render and built only where someone is, the way Voxel Plugin
builds collision around invokers. Measured in WASM, a footing costs 2 us and a
step, which asks for four, costs 8. There is nothing there to cache: a
collision mesh would hold exactly what `Generator::column` already answers
from, and a cache of something costing microseconds is not an optimisation, it
is a second truth to keep in sync with the first. What will earn a
representation is the edit: the day a stored chunk carries one the field is no
longer the whole truth, and collision reads `chunk(addr)` like every other
reader. The invoker idea comes back then, with something to hold.

**The rules, and each is one number.** A rise of one block is taken in stride
and two is a wall, to be jumped or flown, going up and coming down alike. A
body needs its own height of room to walk into a place, except that a body
already under a low roof keeps whatever room it had, so a tight place is never
a place to be stuck in. Blocked, a step is tried along one address axis and
then the other, which is what slides a body along a wall instead of sticking
it to one.

**Rock at the knee means in, not on.** Standing exactly on the ground and
standing buried in it read the same at the feet, because the density is zero
on the surface. Half a metre up they do not. So a footing asks at knee height:
rock there means the body is inside rock, and then the floor is the ground
itself, which is solid and above. One rule does three jobs: a wall taller than
a footing looks is not walked into, a body that got buried is let out upward,
and nothing falls through the planet however fast it arrived.

**Probes are half a cell.** A cell of the volume is one block, and a slab one
cell thick is drawn, so a probe as coarse as a cell straddles it and a body
falls through a ledge it can see. The test that found this walked eight
directions out of a cave mouth and came down through a half metre lip.

**Collision is the ground in full detail**, never the filtered one (29): what a
body stands on may not change with how far away the camera is. The drawn mesh
is band limited and the two differ by less than a cell, which is the same
difference the heightfield walker already lived with.

**The camera goes into the cave.** A third person boom was kept over the
terrain as drawn, so that nobody looks at the ground from underneath. Inside a
cave that rule points the wrong way and would yank the camera out through the
roof, so there the boom is cut by the rock behind it instead: the camera stays
in the cave, as near the body as the walls allow.

**What this does not fix, and it shows.** Collision reaches as deep as the
field does; the volume is drawn about 8 m under the lowest ground of a patch,
which is what a frame affords. A shaft deeper than that drops a body out of
what is drawn, into a world that is hollow under its shell. The fix is the job
queue M1.5 already asks for, not a change here, and until it lands the honest
statement is that a cave is walked from its mouth.

Rejected: a collision mesh built around invokers (nothing to cache until edits
exist); collision against the drawn mesh (a client's LOD would decide where
the ground is, and the server is authoritative by address); holding a body at
the depth the volume happened to be drawn to (the ground would then depend on
what a client had loaded, which is the same fault in a smaller coat).

## 48. The world is voxels to the core, and the heightfield goes (proposed)

The ground has been a heightfield since the first planet: a quadtree of
patches, sampled from `sample_at`, beautiful from orbit and cheap on a
Raspberry Pi. Entry 47 and the two before it bolted a volume onto its deepest
level so that a cave could exist. This entry proposes finishing the job the
other way round: the world is voxels, and a height is at most a way of
summarising them from far away.

**What the hybrid actually is.** A heightfield is a surface of no thickness.
The volume layer is a 16 m window of density meshed where the quadtree runs
out of levels; inside it the world is solid, and one metre below it there is
not rock, there is nothing. Walk into a shaft and you come out under the
world and see the sky through the ground. Every attempt to fix that was the
window again: deeper, then a ring of patches around the body, then a cap on
the chunks one patch may mesh, each one a constant tuned against a bench. That
is the signal a design is wrong rather than unfinished, and the work is parked
on `feat/voxel-volume` instead of continued.

**It is not a culling fault.** In a voxel world a face exists only between
solid and air, so a closed world cannot be seen from the wrong side. What
looked like backface culling was the absence of an interior.

**Orbit is a requirement, not a later concern.** A world has a planet, a moon
and whatever people put in orbit, and all of them are voxels. So the question
is not whether voxels can be near the player, which is easy, but whether a
body made of them can be drawn whole from outside. The arithmetic says yes and
says what it costs: a body of `2^n` blocks a sector side has
`6 * (2^n / 16)^2` surface chunks, which for today's 20.9 km planet is 100
million, and a pyramid of reduced chunks reaches a drawable count in eight or
nine levels, one cell of the coarsest being 128 m. That is the same depth the
heightfield quadtree already runs at. The difference in cost is that a
heightfield level is sampled and a reduced chunk is built, and it has to keep
the mean or a hill grows and shrinks as the camera moves (43).

**The rule the hybrid got wrong, stated properly.** A far level may be hollow,
because nobody can be inside it. The near field must have an inside, as deep
as anybody can go, which is the build band and not a window chosen against a
frame budget. Then every level boundary is volume to volume, and nothing hands
over from a solid to a surface.

**Smaller worlds stop being a nicety.** `SECTOR_BITS` is a compile time
constant, so every world is 20.9 km. It becomes part of the recipe, frozen per
world like the generator version. That is what makes the design provable: at
`2^10`, a radius of 326 m, every level fits and the whole pyramid can be
judged before it is asked to hold a planet. It is also the first real piece of
work, because it touches the address, the generator, the client and the wire
shape.

**What survives, and it is most of it.** `topology` is already the article's
design and arrived there on its own: quad sphere with a pre-distorted mapping
(25), addresses, neighbours across seams and corners, property tested.
`voxel` has the chunk blob and surface nets. `worldgen` has deterministic
density with caves and the band limiting discipline. `render`, `avatar`,
`scene`, the command and event seam, the web app and the server do not care
whether the ground is a height or a volume. What goes is `client::terrain`,
`client::volume`, and every rule that exists because the ground was a height.

**The reference.** `https://bowerbyte.com/posts/blocky-planet/`, which is
where this project's shape came from and which solves the same geometry for a
planet small enough to need no LOD at all. Its shells, which keep a block's
width roughly constant with depth by quadrupling the blocks per layer, are the
one part of its design we named in the glossary and never built.

The brief is docs/VOXEL_BRIEF.md. What it deliberately does not settle is in
OPEN.md: the size the design is proved at and the size a world ships at, how
far orbit has to reach before a level may be a shell, when shells arrive, and
whether a visitor first sees blocks or a smooth surface.

Rejected: keeping the hybrid and widening the window again (the complaint that
opened this, and four tuned constants deep); dropping voxels and going back to
the smooth planet that worked (VISION says a world made of voxels, and
building and digging are the point, not decoration); holding a body at the
depth a client happened to mesh, which makes the ground depend on what was
loaded and is the same fault as 47 rejected in a smaller coat; a voxel near
field with a heightfield far field kept as the permanent answer, which is what
is being abandoned and would only postpone the same seam.

## 49. A world has a size, and a grid has a grain (decided)

`SECTOR_BITS` was a compile time constant, so a world could not be small and
the design could only ever be judged at 20.9 km, where no level of anything
fits in one sitting. It is now `sector_bits` in the recipe, frozen per world
exactly as the generator version is, and for the same reason: it decides the
address, and the address is the save format.

**The range is `4..=16`, and both ends are structural.** 16 because `u` and `v`
fill a `u16`, which is the address we already write. 4 because a sector must
hold a chunk: below that the unit of storage no longer tiles the grid it lives
on. A body of `2^4` is 5.09 m of radius and roughly two chunks across, which is
small enough that the 27 chunks around one of them collapse to 15 distinct
ones. That is the point: a world where every part of the design is visible at
once.

**A grid and a body are different things.** Seams, neighbours and the tangent
warp are pure arithmetic that never needed to know what a cell is, so they
belong to a `Grid`: six square faces at one grain. A `QuadSphere` is one body,
a block grid plus the radius that turns cells into metres. The payoff is not
tidiness: the chunk grid is the block grid coarsened by `CHUNK_BITS`, so a
chunk stepping over a seam is a block stepping over a seam, and the property
tests that were written once for blocks now hold for chunks and, later, for
every reduced level. The properties run at every grain from one cell per face
to `2^16`, and two of them were wrong at the coarse end until they did.

**The band is a human measure, bounded by the core.** Half a band is
`min(256 blocks, radius / 4)`. The 256 blocks are a cellar and a tower, which
are the same depth on a 20.9 km body and on a 300 m one, so the band is not a
fraction of anything. The quarter of the radius is the hollow core keeping its
share, and on a body too small to hold a human band the radius wins. At `2^16`
this is exactly the +-128 m the band has always been, so nothing about today's
planet moves.

**The band follows the surface, not the datum.** ARCHITECTURE always said so;
the first implementation here hung it off the datum and a legitimate vertex at
281 m read as a violation, because the relief reaches +-523 m while the band is
128 m thick. A column under a mountain and a column under a trench hold their
chunks at different heights, and `terrain::band_h` is where that is said.

Rejected: a compile time constant behind a feature or an env var, which would
have cost nothing today and made it impossible for one client to hold a planet
and a moon of different sizes, and would have left the size out of the save
format where it belongs; carrying the size inside `Column`, which grows the
address, duplicates one fact per cell, and lets two bodies' integers be mixed
silently; a band that is a fixed fraction of the radius, which gives a 300 m
world a 2 m band and nothing to build in.

## 50. A world's size is the scale its shape is printed at (decided)

The generator samples the unit sphere by direction and answers in metres, so
the same seed gave the same heights whatever the body was. At `2^4` that is
relief 125 times the radius: a 5 m planet with its ground 523 m below it.

The fix is not inside the generator. A version is written once, in the metres
of the **reference body** - the largest quad sphere there is - and frozen
there forever, because a world's terrain may never shift under its builds.
The body's size is a scale applied at the `Generator` boundary: metres going
in are divided by it, metres coming out are multiplied. Footprints, cave
depths and ground heights all cross that one seam.

**The factor is `side / 2^16`.** Both sides are powers of two, so it is exact,
and at the reference size it is exactly `1`. Multiplying by one changes no bit,
so the largest world is untouched and the golden hashes hold without being
regenerated, which is the only acceptable outcome for a frozen generator.

The consequence is the good one: a small world is the same world printed
smaller. The seed keeps its coastline, its plates and its mountains, and they
arrive at the body's own scale, relief holding its share of the radius at every
size. A world proved at `2^10` is therefore the world that ships at `2^16`,
which is what makes proving it there worth anything.

Rejected: scaling only the height that comes back, which leaves the cave field
at 20.9 km inside a 5 m body; teaching each generator version its body's
radius, which rewrites frozen arithmetic and breaks every golden hash for a
number those versions never used; a relief knob chosen per size, which is a
constant tuned against a picture, the exact failure 48 was written about.

## 51. A frame's budget counts chunks made, not meshes finished (decided)

The voxel streamer blew the frame budget in WASM and would not come down. Four
chunks a frame cost 20 ms against a budget of 12; one chunk a frame cost the
same 20 ms. That the number made no difference was the whole finding.

A mesh reads one cell past its chunk on every side, so a chunk at the frontier
pulls in the 27 around it. `warm` generated all 27 before returning, and a
budget checked between chunks could only ever stop on a multiple of that. The
granularity of the work was 27, and the budget was a fiction.

Now `warm` takes a limit, generates what it can and says whether the
neighbourhood is whole. A chunk that is not ready stays at the front of the
queue and the next frame carries on where this one stopped. The budget counts
chunks **generated**, because that is the work; counting meshes finished would
let one frame do twenty times another's.

Measured, `bun run bench`, ten second descent: worst frame 20.98 ms before,
3.75 ms after, median 1.89, none over budget. The walk through cave country
went from 20.23 ms to 4.57.

Three things were optimised before this was found, and none of them was the
fault: sorting the queue by a key worked out once instead of per comparison,
keeping a chunk's position beside it instead of folding a seam to get it, and
skipping the fold for border cells that never leave their sector. They are all
kept, because each is right, and none of them moved the number. What found it
was the bench printing which frame was worst and how many were over, and an
experiment with the budget set to zero.

Rejected: raising the frame budget, which is how a design stops being measured;
meshing without the border, which opens the seam 48 is about; a fixed cap on
chunks a frame with no relation to what a chunk costs, which is the tuned
constant that decision warns against.

## 52. The far field is the same grid, coarsened (decided)

Pull the camera back and the world ended: there was one level, it reached 56 m,
and past that there was nothing. A body has to be drawable from any distance -
the moon is seen from the Earth - and the ground under the avatar's feet most
of all.

**A level is a grain, not a different thing.** A chunk of level `L` holds cells
`2^L` blocks wide. Its cells live on the block grid coarsened by `L`, and the
chunks on that grid coarsened again by `CHUNK_BITS`. Everything that resolves a
seam, a neighbour or a direction already took a `Grid` and did not care how
coarse it was (49), so the pyramid inherited all of it, and the property tests
that run at every grain from one cell per face upward were already covering the
levels before any existed.

**A coarse chunk is generated, not built.** The brief left both open: build a
reduced chunk from the eight under it, or ask the generator at a coarse
footprint. The generator already takes `footprint_m` and fades out what a mesh
cannot carry (29, 43), so asking it is both cheaper and the band limited
answer. Building from below becomes necessary when chunks are stored and edited,
because then the fine chunks hold something the recipe does not; that is the
same problem as one read path per level and is not solved here.

**A level wants a shell.** Close enough that the level is worth drawing, far
enough that the level under it already covers the ground. `DETAIL` is the only
knob, and it is honest about its cost: every level is a square
`2 * DETAIL + 1` chunks across, so raising it raises the work at every level at
once. The coarsest level has no outer edge, which is what makes a body
undisappearable.

**A level is reworked only when the eye leaves its own chunk.** At level 0 that
is every 8 m; at level 12 it is every 32 km. Without this the scan would be 13
levels deep every time the avatar took a step.

Measured at `2^10`: 1,103 chunks drawn and the whole planet visible from 900 m
up in 73 of them. `bun run bench`: worst frame of a descent 5.68 ms of a 12 ms
budget, median 2.16, none over. Settling a world from nothing at `2^16` is
7.4 s of work spread over frames, and holds 16,642 chunks.

Also fixed here, and it was a plain fault rather than a missing feature:
streaming was centred on the camera, so pulling the boom back unloaded the
ground under the avatar. It is centred on the body now; the camera only looks.

Rejected: raising the reach of a single level, which is the window 48 is about
wearing a larger coat; building reduced chunks from the level below, which is
right once chunks are stored and wrong while they are not, because it costs the
whole pyramid underneath to draw the top of it; skirts to hide level
boundaries, not because they are beneath us but because nothing has yet shown a
boundary that needs hiding, and a fix with no fault to point at is a constant
waiting to be tuned.

## 53. Nothing comes down before its replacement is up (decided)

Crossing a level boundary opened a hole. The streamer dropped a chunk the
moment the pyramid stopped wanting it and queued the chunk that replaces it,
and the queue is drained a few chunks a frame: measured, the ground under the
eye was missing for 37 frames of a descent.

A stale chunk now stays drawn and joins a retiring set. It comes down when the
ground it covered is drawn at the level that took over: its parent when the eye
moved away, its eight children when it moved closer, because the shells that
meet are always one level apart. When neither is coming - the band moved, the
world was left behind - the queue running dry says so.

The cost is holding both for as long as the handover takes, which is the price
of the handover being invisible and is the right way round.

What makes this stay fixed is a test rather than a screenshot: a descent from
three radii to the ground, a step at a time, with the frame budget a real frame
has, asserting the ground under the eye is drawn at some level the whole way.
It reports 37 missing frames against the old behaviour and none against this
one. A picture cannot see a hole that lasts half a second.

Rejected: building the replacement before removing, synchronously, which is the
stall the budget exists to prevent; a fade between levels, which hides a
handover rather than ordering it and needs the renderer to know about levels.

## 54. Exactly one level draws any piece of ground (decided)

The shadows were wrong in a way that pointed at the terrain, not at the shadow
code: wide straight bands lying across open ground with nothing above them to
cast one. The terrain had a corduroy ripple on every slope at the same time,
and both were the same fault.

Levels overlapped. A level's shell started inside the reach of the level under
it, by about two and a half chunks, so in that band two surfaces of different
grain were drawn in the same place. They fought for the depth buffer - that was
the ripple - and both cast into the shadow map - that was the bands.

The overlap was on purpose, to be sure of no gap. The right way to be sure of
no gap is not to leave slack in a distance test, it is to cut the levels out of
each other: a chunk whose eight children are all wanted one level finer is
covered, so it is not drawn. That is exact both ways. No overlap, because
covered means covered; no gap, because a chunk is kept unless every piece of
it is taken. It uses the same parent and child mapping 53 needed, so seams
come along free.

The cost is that a level's inner edge moves whenever the level under it does,
so moving 8 m at level 0 reworks every level above. Measured, `bun run bench`:
the worst frame went from 5.91 ms to 8.85 of a 12 ms budget, median 2.36, none
over. That is the honest price and it is paid.

Shadow casters are also now what the pyramid wants and not what is retiring
(53): a chunk held up so the ground has no hole in it would otherwise double
every shadow through the handover.

What is left, and is a different fault: a fine dither on lit slopes, which is
shadow map acne rather than geometry.

Rejected: keeping the overlap and picking a caster per place, which leaves the
depth fight; a fade between levels, which needs the renderer to know what a
level is; widening the shadow cascades, which was never the problem.

## 55. Ground coarser than a shadow texel does not cast (decided)

The planet from orbit was covered in huge black triangles. Not acne: whole
triangles of the coarse levels, shadowing themselves completely.

A chunk of level 10 has cells 512 m across. The furthest shadow cascade has a
texel of about 7.8 m. One such triangle spans hundreds of texels and its depth
varies across a single one by more than any bias can lift, so every one of them
fails its own depth test. Coarse ground cannot be in a shadow map at all.

`scene::SHADOW_CASTER_CELL_M` is the line: ground with cells wider than that
does not cast. Nothing is lost, because a body seen from that far is lit by the
angle of the sun, which is what lights a planet from space anyway.

**What this is not.** There is a fine speckle left on lit slopes near the
ground, and it is not shadow bias. Measured against the same frame rendered
with shadows off, the speckle the shadow pass adds is 14.7 with the depth bias
we have and 14.99 with Veloren's technique of casting only the faces turned
away from the sun and no bias at all. Front face casting was tried because a
shipped Rust voxel game does exactly that; on our ground it changed nothing and
was reverted rather than kept for looking principled. Only a bias eight times
larger removes the speckle, and that one detaches shadows from what casts them.

So it is not the shadow. It is small scale geometry shadowing itself, which
means the surface nets vertices are rough at sub-cell scale, most likely from
the density byte saturating past `DENSITY_REACH` on steep slopes. That is a
`voxel` question and it is open, in OPEN.md and nowhere else.

Rejected: a bias large enough to hide it, which trades a speckle for shadows
that float off their hills; widening the cascades, which was never the problem
and costs texels everywhere.

## 56. What Dust has to teach us, and what it does not (noted)

`refs/dust` is a Rust voxel engine that ray traces a VDB style tree with
hardware ray tracing. Its requirements rule it out for us: it needs
`VK_KHR_ray_tracing_pipeline` and `VK_KHR_acceleration_structure`, Rust
nightly, and Bevy, which 01 and the M1 discipline both refuse. A Raspberry Pi,
a phone and a browser tab have none of that.

**The part that does not transfer, and is worth naming anyway.** Dust does not
mesh. Every problem this week has been about turning voxels into triangles:
cracks between levels, borders a mesh has to read, two surfaces fighting for a
depth buffer, shadow acne from sub-cell geometry. A renderer that marches a ray
down a hierarchy has none of them. That is a real fork in the road and we are
not taking it, because our targets cannot.

**The part that does.** Its internal node holds, per child slot, either a
pointer to a child or a single value standing for that whole subtree, with a
bitmask saying which (`crates/vdb/src/node/internal.rs`). That is our
`Chunk::Uniform`, except VDB has it at every level of the tree rather than only
at the chunk.

Measured against that idea, seed 1: of the chunks held around a body, 20% say
one thing at `2^8`, 32% at `2^10` and **46% at `2^16`** - 7,785 chunks of
16,924. Every one of them was generated, 256 generator columns each, to find
out it was empty. That is roughly two million column samples spent on nothing
and a large share of the 7.5 s a world takes to settle.

We cannot take the shortcut yet, and the reason is worth writing down: deciding
a chunk is uniform without generating it needs a **bound** on the ground inside
its footprint, and the generator has no such thing for a generated world. A
field carries `ruggedness_m`, which is exactly that bound for a baked one. A
generator that answered "the ground here is between these two heights" would
collapse half the pyramid into values on a parent, and would also be what makes
a far level honestly hollow (48).

That is the lesson, and it belongs to `worldgen` rather than to the renderer.
OPEN.md carries it.

## 57. What Veloren has to teach us, and what it does not (noted)

`refs/veloren` is a Rust voxel game people play, GPL-3.0-or-later, read for
architecture and never copied. It is the counter-example to 48, and it is worth
saying plainly what it chose: its default `terrain_view_distance` is **10
chunks**. Real voxels for about 320 m, and a heightmap for everything else. A
shipped game with a full voxel world decided not to build a voxel pyramid, and
56 already suspected this was the reading that would pay.

**The far field is textures, not chunks.** Three of them: `t_alt`, a 16 bit
height; `t_horizon`; `t_map`, a colour. One mesh, fixed, sampled in the vertex
shader (`lod-terrain-vert.glsl`, `include/lod.glsl`). Nothing streams, nothing
is a quadtree node, nothing cracks, nothing pops, nothing needs a frame budget.
The whole far world is a few megabytes of texture.

We already bake exactly this. A `Field` is a cube map of a body's ground per
sector (44), used today as generator input. It is also the far field, and
`ruggedness_m` is the bound on the ground inside a texel that 56 said the
generator did not have. Baking a generated world's field at creation gives, in
one move, the far field, the bound, and the end of the 7.4 s settle and the 46%
of chunks generated to discover they were empty.

**The horizon map is the answer to 55.** Per texel, the angle of the horizon in
two directions; `horizon_at2` reads it. Large scale terrain self shadowing at
any distance with no shadow map at all. 55 had to stop coarse ground from
casting because one of its triangles spans hundreds of shadow texels and fails
its own depth test. It stopped there. This is where it continues: coarse ground
does not cast into a cascade, it carries its own horizon.

**A smooth surface can be made to look voxel per pixel.** `lod_voxels` marches
a short ray against an implicit grid whose cell size grows with distance, and
returns a cubic normal and an AO term. The far heightfield reads as the same
world as the near cubes without being the same data. That is the seam of a
mixed design made invisible in the fragment shader rather than in the mesh,
which is the opposite of where we spent this month.

**Light is baked into the mesh, not computed per frame.** Sunlight floods
through air at mesh time losing a step per cell (`SUNLIGHT` is 24 in
`mesh/terrain.rs`), and is packed into the vertex beside a glow channel, five
bits each. Per face ambient occlusion comes from the four neighbours of a face
corner. Both are chunk local, both run on a worker, both cost the frame
nothing, and together they are why an interior in that game reads as an
interior. This is what a build volume needs and what no amount of cascade
tuning gives.

**Small things worth stealing.** Indices sorted by altitude, so one mesh is
drawn as the range the eye's depth selects, which is free culling for a cave.
`pull_down`, which sinks the far sheet where the near field takes over so the
two never fight for the depth buffer: blunt, and it works. Instanced LOD
objects for trees and houses, which is what a distant volume should be.

**What does not transfer.** The splay: their far mesh is a fixed grid stretched
radially around the eye, which works in a flat world with a finite map and has
nowhere to go on a sphere seen from orbit. So our quadtree of patches stays.
What changes is that a patch reads a texture instead of calling the generator,
and that is where the cost was. Also not transferable, and the whole of what we
give up: their world is voxels throughout, so digging anywhere is free.

The strategy this points at is docs/BRIEF.md. What it decides goes in its own
entry when it is applied, not here.

## 58. Nature is a surface, building is volumes (decided)

This retires 48. The world is not voxels to the core: the terrain layer is a
heightfield again, not editable by anyone in world, and cubic voxels exist only
inside a volume, which is an integer address box where building is granted.

**Why 48 was right to kill the hybrid and wrong about what to build.** Its
diagnosis holds: a heightfield with a voxel window bolted to the deepest
quadtree level is two representations of the *same* ground handing over where a
quadtree runs out of levels, and that seam let the sky through. Its remedy,
voxels at every level to orbit, was measured over four weeks and priced: a world
takes 7.4 s to settle and holds 16,642 chunks at `2^16`, of which 46% say one
thing all through and were generated anyway (56); the sea, the grass and the
moon were lost; and a sub cell speckle on lit slopes survived every shadow fix
(55). None of that is payable on a Raspberry Pi, a phone and a browser tab.

**What changes is the kind of seam, not the number of representations.** A
volume meets the terrain at a **containment** boundary: authored, integer, and
decided by a person or by the recipe, never by how far the camera is. Nothing
hands a solid over to a surface because nothing is both.

**Veloren settled it** (57): a shipped voxel game draws real voxels for about
320 m and a heightmap past that, its far field is three textures rather than a
chunk pyramid, and its light is flood filled into the mesh on a worker. Our
`Field` already is that far field, and `ruggedness_m` is the bound on the ground
inside a texel that 56 said the generator did not have.

**What this forbids, and it has to be said plainly.** No tunnel in open
wilderness, no well in the middle of nowhere, no cave the generator made that a
body can walk into. The cave work of 47 and the density path that fed it are
kept in `worldgen` because a volume brush will want them, and `client::collision`
stops walking a density column: a footing is the ground height, which is exactly
what the terrain mesh is built from, so the two can no longer disagree.

**A cave, when there is one, is a shell and a room**: a GLB entity for the rock,
seated into the ground by a stamp, with a small volume inside where people dig.
Authored, never generated. It needs no new subsystem, so caves are deferred
rather than designed in. What a body collides with inside that shell is open.

**A stamp is how anything that is not terrain seats into terrain.** A volume
carries a flatten and blend footprint applied when the ground is sampled, so a
platform is part of the recipe and not an edit. Deterministic, small, server
side, and applied at every level by construction. One read path survives.

Deleted here: `crates/voxel` and `crates/terrain`, which will be written again
from scratch when volumes are built, and `client::tests::caves`, whose feature
is gone from nature. `feat/voxel-wrong-path` on the remote is the archive; a
graveyard directory in the workspace would only keep compiling and keep
entering context.

Rejected: keeping the pyramid and raising the budget, which is how a design
stops being measured; a deprecated crates directory instead of deleting, which
is a museum that costs a build; caves as generator placed volumes from the
start, which designs in a subsystem nothing yet needs.

## 59. Voxelis is read, not taken (noted)

`voxelis` (crates.io, MIT OR Apache-2.0) is a Sparse Voxel Octree DAG in pure
Rust with hash consed shared nodes, one release in April 2025 and one
maintainer. The licence is the friendliest of any reference we have. We are not
taking it, for four reasons in order of weight.

Its headline is compressing an enormous voxel world, and 58 deleted that
problem: voxels now live only inside bounded volumes. It has no stated WASM
support and depends on `rayon`, which on `wasm32-unknown-unknown` needs
`wasm-bindgen-rayon`, `SharedArrayBuffer` and COOP/COEP headers, so a browser
tab and a Pi would be ours to port. Hash consing shares subtrees across the
world, which has no address and no owner, while our invariants are authority by
address, copy on first write storing a whole chunk, an op log that says who, and
per user rollback. And it would replace a small tested crate to buy what we do
not need.

What is worth reading: its batch edit, which mutates hundreds of thousands of
cells as one operation for a 22 to 224 times speedup. That is exactly the M3
rule that an op is a gesture and not a cell. Its published per operation numbers
on a 32³ chunk are also a data point for the open question of chunk side.

This is the third reference in the same family after Dust (56) and Veloren (57),
and all three point at the lesson 56 already extracted by measuring: a value on
the parent standing for a whole subtree. We have it. It does not need a
dependency.
