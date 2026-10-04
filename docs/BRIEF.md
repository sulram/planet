# BRIEF: a world mundos hosts, a core and its plugins

The one current campaign, decided in DECISIONS 87 to 91 and 93 to 99. The build layer's
shape is in WORLD.md, its why in DECISIONS 58, and what is left of it to
build in ROADMAP § Plugins.

**There is one brief at a time.** When it ships it dissolves: the shape into
ARCHITECTURE and DEPLOY, the why into DECISIONS, the rest into ROADMAP.

## The decision in one breath

- planet is a world mundos hosts: one world per instance, who is who signed
  by mundos, an update made by copying into the next generation.
- A world is born unfounded and its first admin founds it. Anyone enters;
  admins and builders build.
- The core is the sphere and the host of plugins. What is done on the sphere
  is a plugin, composed at build, switched on and off for a world.
- A plugin is Rust in two halves and a panel. The server hosts its world
  half in a module and holds none of its code.
- Every state has one owner, and plugins meet in the core's words: an op, a
  question, an event. A person and an agent do the same things the same way.
- A world deployable and updatable by mundos comes first. planet's own
  development follows it.

## Order of work

0. [x] **The docs, first**, because they enter every session: VISION,
   ARCHITECTURE, DEPLOY, ROADMAP, OPEN and GLOSSARY at 87 to 91, and this
   brief.
1. [x] **Marlus confirmed 89, 90 and 91**, which step 2 stands on.
2. [x] **A world alone.** The world folder holds the recipe, and an admin
   founds a world that awaits one. The door's token is traded for a key, the key
   opens the socket, and `Welcome` says the level the client offers its tools
   by. The page is static files the Go server sends.
3. [x] **The image**: the `Dockerfile`, its `HEALTHCHECK`, `planet copy`, and
   the release workflow, which published `1.0.0-dev.7` to
   `ghcr.io/sulram/planet` on its first run.
4. [x] **mundos learns the type**, in its own repository (branch
   `feat/planet-type` there). Each type has one adapter: the image's
   repository, the shape of a version, the container's spec, the copy into a
   next generation.
5. [ ] **Walk it.** A planet runs on the box under mundos, founded by its
   admin. Left to walk: entering as anonymous and as a builder, then an
   upgrade into a next generation and its promotion, which wait for plugin
   work worth shipping.
6. [x] **The host of plugins, cut by chat** (98): the wire's envelope, the
   registry and its config, a panel's place. Chat is the first plugin, and a
   world runs with it on or off.
7. [ ] **Chat's world half, and the module** (99): the world half's contract
   in Rust, the bridge between Go and the module over wazero, and chat's
   world half in place of `internal/chat` and `near.go`. A plugin then
   touches no Go.
8. [ ] **Building as a plugin, and kept**: the turn in a frame, the solids and
   the picture through `scene`; then a stroke as an op on the socket, the
   permission hook, the store by volume and its world half in the module
   (93, 95).
9. [ ] **Land**: permission to build by volume, to a person or an agent, the
   second answer to the permission hook (95, 96).
10. [ ] **Avatars**: the figure and the offer of avatars with its hook.
11. [ ] **A second way of building**, a proof of concept: the second
    implementation that makes those seams real (65).

Steps 2 to 4 were the gate: a world is deployed by mundos, and step 6 on
moves. The upgrade of step 5 is walked with the first version that carries
plugin work, on Marlus's word.

## What it keeps

- The frame. `bun run bench` is the referee and the numbers to beat exist:
  worst 3.75 ms of 12, median 1.89. A plugin's turn in a frame is inside that
  budget.
- What a person does today: walk, fly, swim, reach the moon, speak, take a
  name, build in the client. Each survives every step.
- A world that runs alone on a laptop.

## Open questions

These are settled in OPEN.md, their one home: how a plugin's two halves share
its crate, how its panel exists outside Svelte, what a client does in a world
with a plugin it lacks, whose the palette is, and where fields are hosted.
