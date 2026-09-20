# planet

Working codename. A finite, spherical voxel world that people and AI agents
walk, fly, drive and build in together. Free software, built from scratch.

- Why: [docs/VISION.md](docs/VISION.md)
- Shape: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)
- Next: [docs/ROADMAP.md](docs/ROADMAP.md)
- Working rules: [CLAUDE.md](CLAUDE.md)

## Run it

Needs Rust (stable), Go, Bun. Optional: `mailpit` to catch dev email.

```sh
bun run setup      # dependencies, git hooks, wasm target, wasm-bindgen-cli
cp .env.example .env
bun run dev        # server :8090 + web :5173 (builds the engine when missing)
bun run desktop    # the offline explorer in a native window
bun run shot --out out/orbit.png --altitude 30000 --pitch -60 --boom 50
bun run check      # everything CI runs
```

- Sign in at `/login`. With `SMTP_HOST` empty the magic link prints in the
  server log. `PLANET_OPERATOR_EMAIL` may open `/backoffice`.
- Keys: `W A S D` move, `Space` jump, rise or leap from the water, `C` descend or dive, `Shift` run,
  `F` walk or fly, `V` next avatar, `R` new seed (desktop), wheel zoom, `Esc` release the pointer.

## Releases

- Work lands on `dev`: prereleases `0.0.1-dev.N`.
- Merging `dev` into `main` publishes the plain version.
- Conventional Commits decide the bump: [release.config.js](release.config.js).
