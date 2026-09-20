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
left at zero from its smooth days. The moon gained a few wide basins, flooded
with dark mare through a continuous `Sample::shade`, so it reads from the
planet.
