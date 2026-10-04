# apps/web: rules

The root `CLAUDE.md` applies here too. Plataforma ITS (`refs/its-plataforma`)
is the model: SvelteKit 2, Svelte 5 runes, Vite 8, the same versions.

- Every reusable UI element enters `$lib/ds` before a second use, with the
  `/ds` catalogue in the same commit. Components use semantic tokens only.
- No user visible literal string: `t('key')`. `en.ts` is the source of truth.
- Every wait gives a signal (`loading`). Destructive actions confirm in `Dialog`.
- The app is a front end and nothing else (DECISIONS 87, 90): static files,
  and no route that needs a server (no `+page.server.ts`, no
  `hooks.server.ts`, no form action). What it asks, it asks the world's routes
  through `$lib/instance.ts`; what it keeps (language, theme, avatar, a
  visitor's name), the browser keeps.
