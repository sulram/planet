# apps/web: rules

The root `CLAUDE.md` applies here too. Plataforma ITS (`refs/its-plataforma`)
is the model: SvelteKit 2, Svelte 5 runes, Vite 8, the same versions.

- Every reusable UI element enters `$lib/ds` before a second use, with the
  `/ds` catalogue in the same commit. Components use semantic tokens only.
- No user visible literal string: `t('key')`. `en.ts` is the source of truth.
- Every wait gives a signal (`loading`). Destructive actions confirm in `Dialog`.
- A plugin's panel lives in the plugin's own folder, `plugins/<name>/web/`
  (DECISIONS 100), a package this app depends on. It reaches the engine
  through its `Seam` alone (`$lib/plugins/plugin.ts`); the stage mounts its
  layer while the world has it on. `$lib/plugins/index.ts` is generated.
- The app is a front end (DECISIONS 87, 90): static files, each route built
  once as a shell the browser fills. What it asks, it asks the world's routes
  through `$lib/instance.ts`; what it keeps (language, theme, avatar, a
  visitor's name), the browser keeps.
