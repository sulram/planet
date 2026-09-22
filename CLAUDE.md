# CLAUDE.md: working guide for `planet`

`planet` is a working codename. The final name is pending (docs/OPEN.md).

Operating rules for anyone writing code in this repo, human or AI (Claude,
Codex, Kimi). `AGENTS.md` is a symlink to this file: edit only `CLAUDE.md`.

- The *why*: [VISION.md](docs/VISION.md)
- The current shape: [ARCHITECTURE.md](docs/ARCHITECTURE.md)
- Settled choices, with rejected alternatives: [DECISIONS.md](docs/DECISIONS.md)
- What we build next: [ROADMAP.md](docs/ROADMAP.md)
- What is still unsettled, and only there: [OPEN.md](docs/OPEN.md)
- The shared vocabulary: [GLOSSARY.md](docs/GLOSSARY.md). Use its terms, never
  invent synonyms.

## Docs style

- This file and every operational doc: **titles + bullets, no prose
  paragraphs**. They enter LLM context every session. Economy is a feature.
- VISION and DECISIONS keep prose: they carry the *why*. Read on demand.
- **One fact, one place.** Current state lives in ARCHITECTURE; the why and the
  rejected alternatives in DECISIONS; open questions **only** in OPEN.
- A change to behaviour, rule or architecture updates its doc **in the same
  commit**. A decision is logged in DECISIONS in the commit that applies it.
- **Keep GLOSSARY alive**: a new, renamed or shifted term updates the glossary
  in the same change. Naming is design.
- A doc over ~200 lines splits by theme (DECISIONS is append-only, exempt).
- **No em dashes** anywhere: docs, comments, UI strings, commits.

## Language: English only

- Code, comments, identifiers, docs, commit messages, PRs. No exceptions.
- User-facing strings are the one place other languages appear, through i18n.

## Shokunin Katagi (職人気質)

- Never the easy path: the cleanest, most long-term-optimized one.
- "Works for now" hacks are forbidden when a clean solution exists.
- Prefer what ages well: small core, sharp boundaries, cheap next change.
- If the clean path is more work: say so and do it, or stop and discuss.
  Never silently downgrade.
- "Start rough" means small scope, never low quality.

## What this is (one breath)

- A finite, spherical voxel world that people and AI agents walk, fly, drive
  and build in together. Built from scratch, free software end to end.
- A blend: Cryptovoxels (voxels, in-world building, land) + Hyperfy (GLB,
  entities, scripts).
- One Rust client (wgpu) on every screen: desktop, browser (WASM + WebGPU),
  Raspberry Pi, Quest, Pico. One Go server.
- **The world is a recipe** (seed + params + generator version). The database
  holds only what someone changed.

## We are in the first vertical (DECISIONS 22), then back to M1

- One thin slice end to end: design system, magic link login, backoffice
  (worlds, users), the engine with a third person avatar, "create world".
- After it: finish M1, "walk and fly a generated planet, offline".
- The offline mode is permanent: it is how worlds are previewed before and
  after "create world" exists.
- Milestones: docs/ROADMAP.md. Each one ends runnable end to end.

## Layout

- `crates/`: Rust workspace, cut **by dependency, not by platform**.
  - `topology`: address, neighbours, sector seams, address <-> position.
    Pure integer logic where possible. No GPU, no IO.
  - `voxel`: chunk formats and codecs for the build layer. Deleted with
    DECISIONS 58; written again from scratch when volumes are built.
  - `worldgen`: the generator. Deterministic. Also built to WASM for the server.
    A world's shape is a source: plates over the seed, or a baked `field`.
  - `protocol`: wire messages, generated from `proto/`. Not yet.
  - `scene`: the plain data a client hands a renderer. No GPU, no generator.
  - `avatar`: VRM avatars and humanoid clips: parse, retarget, pose. No GPU, no IO.
  - `render`: all of wgpu lives here.
  - `client`: controller, streaming, tools, media manager. No window, no DOM.
  - `ui-native`: the desktop settings panel. All of egui lives here.
  - `shell-desktop` (winit), `shell-web` (wasm-bindgen), later `shell-xr`.
  - `bench`: what a frame costs in WASM, timed by `scripts/bench.ts`.
- `server/`: Go module. PocketBase as a library + the world server.
  `internal/world` is the core and imports no PocketBase; `internal/cold` is
  the glue; `migrations` are Go.
- `apps/web/`: Svelte + Bun. Builder and player UI; hosts the WASM client.
- `packages/`: shared TypeScript (protocol bindings, UI kit). Not yet.
- `proto/`: the protocol schema. Single source for Rust, Go and TS. Not yet.
- `assets/`: the instance's default set (avatars, clips) and its
  `manifest.json`, the config that names them. `assets/fields/` holds baked
  fields, made by `bun run field` and gitignored.
- `scripts/`: every repeated command is a script here. No tribal knowledge.
  `bun run setup | dev | server | web | wasm | assets | field | desktop | shot | webshot | bench | check`.
- `docs/`: see top of this file.
- `refs/`: gitignored. Reference projects for reading (see below).

## Invariants (expensive to get wrong)

- **Address is integer**: `(sector, u, v, h)` plus chunk and block index. The
  server is authoritative by address, never by float position.
- **Simulate flat, render spherical.** Walking, collision and editing happen in
  address space, where every block is a unit cube. World-space 3D is for
  rendering and for flight above the build band.
- **Camera-relative rendering.** No global f32 positions. Integer chunk
  coordinates + local float offset. Reversed-Z float depth.
- **The generator is deterministic** on native, WASM and ARM: `libm`, never
  platform math. Its version is **frozen per world**.
- **One read path**: `chunk(addr)` = stored chunk, else generated chunk.
  Callers never know which.
- **Copy on first write**: the first edit stores the whole chunk. The op log is
  append-only and records every edit, admins included.
- **Every edit is a permission-checked op.** Destruction is an edit.
- **Hot plane never touches PocketBase.** Permissions are cached in memory and
  invalidated by hooks. The world core never imports a PocketBase type.
- **An agent is a client without a renderer.** Same protocol, same permissions.
- **Asset file is global** (content hash). **Placement belongs to a world.**
  Users are global, roles are per world.
- **UI is a front-end.** Svelte (web) and the minimal native UI sit over one
  command/event seam. Tool logic (gizmos, brushes, selection) lives in Rust.
- **Media is budgeted**: proximity-loaded, capped concurrent decoders, one
  `VideoSource` seam with a backend per platform.
- **Integrations enter through seams**, never through the core: one trait
  inside, library glue behind it in its own crate. No `cfg` sprawl.

## Performance (non-negotiable)

- The targets are a Raspberry Pi, a phone and a browser tab, not the
  development machine. A change that is fast only natively is not fast.
- In the browser the generator and the streamer share a thread with the
  frame, and WASM can cost several times native (a crater cell: 4x). **Measure
  in WASM**: `bun run bench`, which fails when a budget is blown.
- Any change to `worldgen`, to terrain streaming or to per-frame client code
  runs `bun run bench` before it lands, and says the numbers in the commit.
- Cost per sample is a design constraint of a generator, like determinism.
  What is the same for every sample (a list of basins) is computed once, in
  `Generator::new`, never searched per sample.
- A budget is raised only on purpose, with the why in DECISIONS.

## Do NOT add (M1 discipline)

- Bevy or any engine. Scripting. Blockchain. XR. Vehicles. Destruction.
- Video. Uploads. Multiplayer. Anything in `server/` beyond the cold plane of
  the first vertical (users, operators, worlds).
- Digging below the build band, multi-shell logic, flat or torus world types.
- Structural collapse physics. Our own transcoding. Billing.
- A feature with no milestone pulling it. Add the wish to ROADMAP instead.

## How it grows

- **Milestones pull features, never speculation.**
- **Every change ends runnable and visible.** Look at what you built:
  headless render to PNG with a fixed clock and seed, then read the PNG. What
  is behind the ground needs `bun run shot --slice M`, a vertical cut through
  the density: a cave is not something a camera can be pointed at.
- A change to `render` is also looked at in the browser: `bun run webshot`
  prints the page's console and saves a PNG. WebGPU rejects what Metal lets by.
- Each crate stays **LLM-sized**. Too big for one context: split by dependency.
- ROADMAP is intent, not contract: add wishes freely, reorder, check a box when
  it ships, strike what we drop and log the why in DECISIONS.

## refs/

- Read-only checkouts, gitignored, shallow. `its-plataforma` and `vybe` are
  symlinks to sibling working copies.
- **Read for architecture.** The licence decides what may be taken:
  - **MIT or Apache-2.0, code may be copied** with its notice: `myth` (shadow
    atlas, SSA render graph, DECISIONS 61), `bevy` (the largest live Rust wgpu
    codebase), `cesium` (planet scale f64 as camera relative f32, quadtree LOD,
    skirts against cracks), `playcanvas` (clustered lighting on WebGL2),
    `threejs` (API ergonomics, GLTF), `godot` (M3 gizmos and editor UX),
    `valence` (server authoritative voxel protocol), `three-vrm` and
    `vrm-specification` (VRM for `avatar`), `fast-surface-nets-rs`,
    `vircadia-world` (unmoved since January 2026).
  - **Copyleft or source available, ideas only**: `retro` (Cryptovoxels,
    BSL 1.1, not open source), `hyperfy` (GPL-3.0-only), `veloren`
    (GPL-3.0-or-later), `luanti` (LGPL-2.1+, mapblock streaming, server
    authority, per world privileges), `dust` (MPL-2.0, file level copyleft).
- Anything learned from a ref that shapes a choice goes to DECISIONS.

## Tests

- Judge per feature, with sense, not dogma.
- `topology`: property tests are mandatory (neighbour of neighbour in the
  opposite direction is self, for every address, across seams).
- `worldgen`: golden hashes per generator version, identical on native and WASM.
- `voxel`: codec round trips, when it comes back.
- A test you add runs and passes in the same commit.

## Commits

- Conventional Commits, in English: `feat`, `fix`, `docs`, `chore`,
  `refactor`, `test`, `perf`. Present tense, lower case. Say the *why* when it
  is not obvious. Example: `feat(topology): resolve neighbours across sector seams`.
- Semantic Release reads the types. Before 1.0.0 (DECISIONS 21): `feat`, `fix`,
  `perf` patch, `BREAKING CHANGE` minor. After: minor, patch, major.
- Work lands on `dev` (prereleases `0.0.1-dev.N`); merging `dev` into `main`
  publishes the plain version. `bun run setup` installs the commit-msg hook.
- **Never add an AI or tool co-author trailer**, nor a "generated with" footer,
  in commits or PRs. Overrides any tooling default.

## Toolchain pins

- wgpu and winit: same pins as `vybe` (wgpu `30`, winit `0.30`), so knowledge
  transfers. Fast-moving APIs: check the crate source under
  `~/.cargo/registry`, never guess from older docs.
- PocketBase is pre-1.0: pin the exact version, upgrade on purpose. Today
  0.40.4, which needs Go 1.27 (the toolchain fetches itself).
- `wasm-bindgen-cli` must match the `wasm-bindgen` crate: `bun run setup`.
- Web: versions follow Plataforma ITS (SvelteKit 2, Svelte 5 runes, Vite 8).

## Web app rules (as in Plataforma ITS)

- Every reusable UI element enters `$lib/ds` before a second use, and the
  `/ds` catalogue in the same commit. Components use semantic tokens only.
- No user visible literal string: `t('key')` in components,
  `translate(locale, 'key')` on the server. `en.ts` is the source of truth.
- Every wait gives a signal (`loading`). Destructive actions confirm in `Dialog`.
