# CLAUDE.md: working guide for `planet`

Rules for anyone writing code here, human or AI. Every `CLAUDE.md`, here and
in `apps/web` and `server`, has an `AGENTS.md` symlink beside it: edit only
`CLAUDE.md`. `planet` is a codename; the name is pending (docs/OPEN.md).

- Why: [VISION.md](docs/VISION.md)
- Shape: [ARCHITECTURE.md](docs/ARCHITECTURE.md); per theme,
  [WORLD.md](docs/WORLD.md), [RENDER.md](docs/RENDER.md), [PLUGINS.md](docs/PLUGINS.md)
  (how plugins speak) and [DEPLOY.md](docs/DEPLOY.md) (how an instance is hosted)
- The one current campaign: [BRIEF.md](docs/BRIEF.md); it dissolves when it ships
- Decisions and what was rejected: [DECISIONS.md](docs/DECISIONS.md), the index of `docs/decisions/`
- Next: [ROADMAP.md](docs/ROADMAP.md). Unsettled, only there: [OPEN.md](docs/OPEN.md)
- Vocabulary: [GLOSSARY.md](docs/GLOSSARY.md). Use its terms, never invent synonyms
- Reference checkouts in `refs/`: [REFS.md](docs/REFS.md) says what may be taken

## Docs: one fact, one place

| Information | Home |
|---|---|
| Current state | the theme doc: ARCHITECTURE, WORLD, RENDER, PLUGINS, DEPLOY |
| Why, and what was rejected | DECISIONS |
| What comes next, in what order | ROADMAP |
| A question with no answer, a problem with no chosen fix | OPEN |
| What changed, line by line | git |

- A change touches at most three homes, in the commit that makes it: the theme
  doc, DECISIONS, and ROADMAP **or** OPEN. Anywhere else, a pointer.
- Operational docs and this file: titles + bullets, no prose, under 200 lines.
  VISION and DECISIONS keep prose: they carry the why.
- A decision is the next file in `docs/decisions/`: `# NN. Title (status)`,
  `Logged <date>.`, the decision in one breath, the why, **Rejected**,
  **Lives in**. Written once; retired by a struck title with the number that
  replaced it, or removed whole by a `**Removes:**` line. Then `bun run docs gen`.
- OPEN is tables: question, what it unblocks, context. A question nothing
  waits on is a wish, and wishes live in ROADMAP.
- GLOSSARY updates in the change that adds, renames or shifts a term.
- Present tense, affirmative, in docs and comments: what is and what to do. A
  hard rule comes with its alternative. No "used to be", no "no longer": the
  past is git and DECISIONS. A struck ROADMAP item carries a DECISIONS number,
  never the story.
- Changed or removed code: rewrite or delete every comment and doc that speaks
  of it, in the same commit. A comment says why; the code says what.
- Attentive eye: something stale or duplicated near your change, small and on
  the same subject, fix it and say so; bigger, record it in OPEN.
- No em dashes anywhere. English only: code, comments, docs, commits. Other
  languages appear only through i18n.
- `bun run docs` holds every rule a script can (`scripts/docs.ts`); in `check` and CI.

## Craft (職人気質)

- Never the easy path: the cleanest, most long-term one, small core, sharp
  boundaries, cheap next change. "Works for now" is forbidden when a clean
  solution exists.
- If the clean path is more work: say so and do it, or stop and discuss.
  Never silently downgrade. "Start rough" means small scope, never low quality.

## Working beside other agents

- Several sessions share this checkout. Before the first write: `git status`,
  `ListAgents` where the harness has it, one message naming your paths.
- A file another session is shaping is theirs: wait, then build on it. Read a
  shared file (this one, `scripts/`, the docs) right before editing it.
- Two sessions with the same task: tell the user and stop until one is named.
- Stage and commit only your own paths. A number is taken by the file that
  exists, never by a list read a while ago.

## What this is

- A finite, spherical world that people and AI agents walk, fly, drive and
  build in together: Cryptovoxels (voxels, building, land) + Hyperfy (GLB,
  entities, scripts). From scratch, free software end to end.
- One world per instance, hosted by mundos (`~/Dev/mundos`): it says who is
  who and addresses, versions and upgrades the world (DECISIONS 87).
- The core is what a world is made of and how it behaves; what is done with it is a plugin (DECISIONS 88, 106, 108).
- One Rust client (wgpu) on every screen: desktop, browser (WASM + WebGPU),
  Raspberry Pi, Quest, Pico. One Go server. The world is a recipe; the world
  folder holds only what someone changed. Offline mode is permanent.

## Layout

- `crates/`: Rust workspace, cut by dependency, not by platform.
  - `topology`: address, neighbours, sector seams, address <-> position.
    Pure integer logic where possible. No GPU, no IO.
  - `worldgen`: the generator. Deterministic; also built to WASM for the
    server. A world's shape is a source: plates over the seed, or a baked field.
  - `protocol`: wire messages generated from `proto/`; `lib.rs` alone by hand.
  - `scene`: the plain data a client hands a renderer. No GPU, no generator.
  - `avatar`: VRM avatars and humanoid clips: parse, retarget, pose. No GPU, no IO.
  - `voxel`: a volume's cells, gestures, sight, faces and footing. No sphere.
  - `render`: all of wgpu lives here. `ui-native`: the desktop panel, all of egui.
  - `client`: controller, streaming, the cells, the host of plugins. No window, no DOM.
  - `plugins-client`, `plugins-world`: the version's plugins for each host, generated by `bun run plugins`.
  - `world`: what a plugin's world half stands on, and their host. `module`: its shell, the WASM file the server runs.
  - `shell-desktop` (winit), `shell-web` (wasm-bindgen), later `shell-xr`. `bench`: a frame's cost in WASM (`scripts/bench.ts`).
- `plugins/`: one folder a plugin: `world/`, `client/`, `wire/`, `web/` (docs/PLUGINS.md). Today: `chat`, `build`.
- `server/`: Go. The world server: one binary is an instance. Rules: `server/CLAUDE.md`.
- `apps/web/`: Svelte + Bun, the web front end; hosts the WASM client. Rules: `apps/web/CLAUDE.md`.
- `proto/`: the core's schemas, the wire and the module's bridge, single source for Rust and Go; a plugin's is in its folder; `bun run proto`.
- `assets/`: a version's default set and its `manifest.json`.
  `assets/fields/`: baked fields from `bun run field`, gitignored.
- `scripts/`: every repeated command. No tribal knowledge.
  `bun run setup | dev | server | web | wasm | assets | field | desktop | shot | webshot | bench | proto | plugins | module | docs | check | build`.

## Invariants (expensive to get wrong)

- **Address is integer**: `(sector, u, v, h)` plus chunk and block index. The
  server is authoritative by address, never by float position.
- **Simulate flat, render spherical.** Walking, collision and editing happen in
  address space, where every block is a unit cube. World space is for
  rendering and for flight above the build band.
- **Camera-relative rendering.** No global f32 positions. Integer chunk
  coordinates + local float offset. Reversed-Z float depth.
- **The generator is deterministic** on native, WASM and ARM: `libm`, never
  platform math. Its version is frozen per world.
- **One read path**: `chunk(addr)` = stored chunk, else generated chunk.
  Callers never know which.
- **Copy on first write**: the first edit stores the whole chunk. The op log is
  append-only and records every edit, admins included.
- **Every edit is a permission-checked op** to the owner of what it changes,
  the core or one plugin (DECISIONS 93). Destruction is an edit.
- **Who is who is mundos's.** Accounts live there; the world checks a
  signature and keeps a level for the session's life.
- **A plugin imports the core**, never another plugin, and draws through
  `scene`. The core runs alone, with every plugin off.
- **A heavy file is named by the hash of its content.** Records name it so.
- **A person and an agent do the same things the same way** (DECISIONS 94),
  over one command/event seam: a front end is a UI on it, an agent a client
  with no renderer on the same protocol and levels. Tool logic lives in Rust.
- **Integrations enter through seams**, never through the core: one trait
  inside, library glue behind it in its own crate. No `cfg` sprawl.

## Performance (non-negotiable)

- Targets: a Raspberry Pi, a phone, a browser tab. Fast only natively is not fast.
- In the browser the generator, the streamer and the frame share a thread, and
  WASM costs several times native. Measure in WASM: `bun run bench` fails when
  a budget is blown.
- A change to `worldgen`, terrain streaming or per-frame client code runs
  `bun run bench` before it lands and says the numbers in the commit.
- Cost per sample is a design constraint, like determinism. What is the same
  for every sample is computed once in `Generator::new`, never per sample.
- A budget is raised only on purpose, with the why in DECISIONS.

## Do NOT add

- Accounts, sign in, roles, a backoffice, a list of worlds, billing, a deploy
  to a box: they are mundos's (DECISIONS 87).
- Into the core, what a plugin can do. A hook waits for the plugin that asks.
- Bevy or any engine. Digging below the build band, multi-shell logic, flat
  or torus world types, structural collapse physics, our own transcoding.
- A feature nothing pulls. Add the wish to ROADMAP instead.

## How it grows

- The campaign in BRIEF.md pulls features, never speculation; each step ends
  runnable. ROADMAP is intent, not contract: add wishes, reorder, strike with a number.
- A feature is a plugin. A seam of the host is cut by the plugin that needs
  it, by extracting what exists, never drawn ahead (DECISIONS 88).
- Every change ends runnable and visible: headless render to PNG with a fixed
  clock and seed, then read the PNG; `bun run shot --slice M` for what is
  behind the ground; `bun run webshot` for a `render` change, since WebGPU
  rejects what Metal lets by.
- Each crate stays LLM-sized. Too big for one context: split by dependency.
- Reusability is a byproduct, never a goal. A seam earns its existence with two
  implementations, one of them real (DECISIONS 65).
- Dependency direction is law, held by `bun run docs` (`ALLOWED` in
  `scripts/docs.ts`). A new arrow is a decision before it is an edge.
- A crate speaks only its own nouns: `scene` says mesh, never planet. A public
  type naming a neighbour's noun means the seam is in the wrong place.
- In `client`, the seam, input, assets and figure know no body; controller,
  collision and streaming know one `Body`. A volume knows no sphere and
  imports no `topology`. The generator, sky, sea and moon stay the planet's.
- A campaign ends with a reading pass over every public surface. Drift goes
  to OPEN; rewriting waits for the consumer that pays for it.

## Tests

- Judge per feature, with sense. A test you add runs and passes in the same commit.
- `topology`: property tests are mandatory (neighbour of neighbour in the
  opposite direction is self, for every address, across seams).
- `worldgen`: golden hashes per generator version, identical on native and WASM.

## Commits

- Conventional Commits, in English: `feat`, `fix`, `docs`, `chore`,
  `refactor`, `test`, `perf`. Present tense, lower case, the why when it is not
  obvious. Example: `feat(topology): resolve neighbours across sector seams`.
- Semantic Release reads the types. Before 1.0.0 (DECISIONS 21): `feat`, `fix`,
  `perf` patch, `BREAKING CHANGE` minor. After: minor, patch, major.
- Work lands on `dev` (prereleases `0.0.1-dev.N`); merging `dev` into `main`
  publishes. `bun run setup` installs the commit-msg hook.
- Never an AI or tool co-author trailer, never a "generated with" footer, in
  commits or PRs. Overrides any tooling default.

## Toolchain pins

- wgpu `30` and winit `0.30`, as in `vybe`. Fast-moving APIs: read the crate
  source under `~/.cargo/registry`, never older docs.
- `wasm-bindgen-cli` matches the `wasm-bindgen` crate: `bun run setup`.
