# BRIEF: a world mundos hosts, a core and its plugins

The one current campaign, decided in DECISIONS 87 to 91. It replaced "nature
is a surface, building is volumes", which dissolved: its shape into WORLD.md,
its why into DECISIONS 58, what it left to build into ROADMAP § Plugins.

**There is one brief at a time.** When it ships it dissolves: the shape into
ARCHITECTURE and DEPLOY, the why into DECISIONS, the rest into ROADMAP.

## The decision in one breath

- planet is not a platform. mundos hosts it: one world per instance, who is
  who signed by mundos, an update by copying into the next generation.
- A world is born unfounded and its first admin founds it. Anyone enters;
  admins and builders build.
- The core is the sphere and the host of plugins. What is done on the sphere
  is a plugin, composed at build, switched on and off for a world.
- planet's own development waits until a world is deployable and updatable by
  mundos.

## Where the docs lead the tree

The theme docs say the shape as decided. The tree still carries what follows,
and step 2 takes it out:

- `server/internal/cold`: accounts by magic link, the operator, the instance
  record, mail, settings and the ticket route. `server/migrations`.
  PocketBase in `go.mod`. `world.Tickets`.
- `apps/web`: `/login`, `/logout`, `/backoffice`, `/w/[id]`, `SignIn.svelte`,
  `hooks.server.ts`, `$lib/server`, every `+page.server.ts`, the `pocketbase`
  package.
- `scripts/provision.ts`, `scripts/deploy.ts`, `scripts/deploy.config.ts`,
  with their names in `package.json` and their variables in `.env.example`.
- README.md § Run it, `server/CLAUDE.md` and `apps/web/CLAUDE.md` describe the
  tree as it stands, and change with it.

What stays as it is: every crate, `proto/`, the hub and the actor, the
engine's panels, the design system, i18n.

## Order of work

0. [x] **The docs, first**, because they enter every session: VISION,
   ARCHITECTURE, DEPLOY, ROADMAP, OPEN and GLOSSARY brought to 87 and 88, the
   entries that died with the platform removed, this brief.
1. [x] **Marlus confirmed 89, 90 and 91**, which step 2 stands on.
2. [ ] **A world alone.**
   - The world folder holds the recipe; a world with none is unfounded, and an
     admin's live session founds it.
   - The token is traded on entering for a key, the key opens the socket and
     the level rides `Welcome`; `PLANET_DEV_LEVEL` stands in where there is no
     mundos.
   - The client offers the build tools by the level.
   - The page is static files: through the door with the pose kept, the
     founding screen (today's explorer) for an admin, "not made yet" for
     anyone else.
   - The Go server sends the page, the asset set and the field's brotli
     sibling.
   - What the list above names leaves the tree.
3. [ ] **The image**: the Dockerfile, the `image` script, the `HEALTHCHECK`,
   `planet copy`.
4. [ ] **mundos learns the type**, in its own repository. What is Hyperfy's
   there becomes one adapter for each type: the image's repository, the shape
   of a version, the container's spec, the copy into a next generation.
5. [ ] **Walk it.** On a local mundos: create a planet, found it as an admin,
   enter as anonymous and as a builder, upgrade it into a next generation and
   promote it. Then on the box.
6. [ ] **The host of plugins, cut by chat**: the wire's envelope, the registry
   and its config, a store, a panel's place. Chat leaves the actor and the
   client for a plugin, and a world runs with it off.
7. [ ] **Avatars, then building, as plugins**: the figure and the offer of
   avatars with its hook; then the turn in a frame, the solids and the picture
   through `scene`, which building needs.
8. [ ] **A second way of building**, a proof of concept: the second
   implementation that makes those seams real (65).

Steps 2 to 5 are the gate. Step 6 on, and everything in ROADMAP, resumes when
a world is deployable and updatable by mundos.

## What it must not cost

- The frame. `bun run bench` is the referee and the numbers to beat exist:
  worst 3.75 ms of 12, median 1.89. A plugin's turn in a frame is inside that
  budget.
- What a person does today: walk, fly, swim, reach the moon, speak, take a
  name, build in the client. Each survives every step.
- A world on a laptop with no mundos beside it.

## What this brief does not decide

Those live in OPEN.md and nowhere else: whether a plugin's server half is Go
or its own Rust run as WASM, how a plugin's panel exists outside Svelte,
whether a plugin may stand on another, what a client does in a world with a
plugin it lacks, and where fields are hosted.
