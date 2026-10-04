# 100. A plugin is one folder: its crate, its wire and its panel together (decided; where its Rust sits is 101's)

Logged 2026-10-04.

A plugin lives in one folder, `plugins/<name>/`: its Rust, its schema under
`wire/` and its panel under `web/`. The folder is a member of the Cargo
workspace and a package of the Bun workspace at once. Outside it, only what
`bun run plugins` generates names it: the registries and buf's list of
schemas. The Rust is one crate at the folder's root, the client half; where
the world half sits beside it is settled when it is built (99, OPEN).

**Why.** Chat was cut through four trees: `proto/planet/chat`, `crates/chat`,
`server/internal/chat` and the web's `$lib/plugins/chat`. Marlus read it as
code spread out, and to see a plugin whole, to review it or to remove it was
to walk four places. With its Go gone (99), what is left of a plugin is Rust,
a schema and a panel, and they change together. The fork has this as a folder
with `client.js` and `server.js` (88). The rule that a plugin imports the
core and never another plugin (93) also becomes a matter of paths: outside
`plugins/`, nothing written by hand points in.

**What was tried before logging**, in a copy of the repository. Cargo loads
`plugins/*` as members beside `crates/*`, and a folder there without a
manifest fails the whole workspace, so every plugin has a crate. The panel,
as a package the web app depends on, is checked by `svelte-check` from
`apps/web`: three errors planted in it were all reported, a wrong i18n key
among them. The page builds with one copy of Svelte. buf lints the plugin's
schema as a module of its own, beside the core's, and generates the same
Rust. Vite's development server refuses a file outside the app until
`server.fs.allow` names `plugins/`.

**What a panel imports.** The web front end's own core: `$lib/engine`,
`$lib/ds`, `$lib/i18n` and `$lib/plugins/plugin`. They are to a panel what
`client` is to a plugin's crate.

**What stays outside the folder.** Chat's server half, `server/internal/chat`,
until its world half is built (99). A plugin's strings, in the web front
end's catalogue. `plugins.json` at the root, which says what a version
carries, where the folder says what exists.

**Rejected:** the four trees, a plugin's parts filed by toolchain; the panel
imported by a relative path with no package, where a file outside `apps/web`
finds no `svelte`; a `buf.yaml` edited by hand for each plugin, where the
config is the one place a person edits (98); the panel moved and the crate
left in `crates/`, half of it.

**Lives in:** PLUGINS.md § Where a plugin lives; CLAUDE.md § Layout;
`Cargo.toml`; `package.json`; `buf.yaml`; `scripts/plugins.ts`;
`scripts/proto.ts`; `scripts/docs.ts`; `apps/web/vite.config.ts`.
