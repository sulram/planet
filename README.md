# planet

Working codename. A finite, spherical voxel world that people and AI agents
walk, fly, drive and build in together. Free software, built from scratch.

- Why: [docs/VISION.md](docs/VISION.md)
- Shape: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)
- Next: [docs/ROADMAP.md](docs/ROADMAP.md)
- Working rules: [CLAUDE.md](CLAUDE.md)

## Run it

Needs Rust (stable), Go, Bun.

```sh
bun run setup      # dependencies, git hooks, wasm target, wasm-bindgen-cli
bun run dev        # the world server :8090 + the page :5173 (builds the engine first)
bun run desktop    # the offline explorer in a native window
bun run shot --out out/orbit.png --altitude 30000 --pitch -60 --boom 50
bun run check      # everything CI runs
```

- Alone, whoever opens the page is an admin. The first visit
  founds the world: a seed, Earth or generated, its knobs. It is kept in
  `server/world`; delete that folder for an unfounded world again.
- Hosted, a world is an image and three variables mundos sets:
  [docs/DEPLOY.md](docs/DEPLOY.md). `.env.example` names them.
- Keys: `W A S D` move, `Space` jump, rise or leap from the water, `C` descend or dive, `Shift` run,
  `F` walk or fly, `V` next avatar, `R` new seed (desktop), wheel zoom, `Esc` release the pointer.

## Releases

- Work lands on `dev`: prereleases `0.0.1-dev.N`.
- Merging `dev` into `main` publishes the plain version.
- Conventional Commits decide the bump: [release.config.js](release.config.js).
