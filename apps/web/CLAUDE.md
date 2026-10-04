# apps/web: rules

The root `CLAUDE.md` applies here too. Plataforma ITS (`refs/its-plataforma`)
is the model: SvelteKit 2, Svelte 5 runes, Vite 8, the same versions.

- Every reusable UI element enters `$lib/ds` before a second use, with the
  `/ds` catalogue in the same commit. Components use semantic tokens only.
- No user visible literal string: `t('key')` in components,
  `translate(locale, 'key')` on the server. `en.ts` is the source of truth.
- Every wait gives a signal (`loading`). Destructive actions confirm in `Dialog`.
- The app is a front end and nothing else (DECISIONS 87): no sign in, no
  backoffice, no rule of its own. What the tree still has of them leaves with
  the campaign in `docs/BRIEF.md`; add no route that needs a server.
