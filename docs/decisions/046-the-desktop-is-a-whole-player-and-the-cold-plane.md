# 46. The desktop is a whole player, and the cold plane moves to the seam (proposed)

Logged 2026-09-21.

The desktop build can render any planet and walk it, and can do nothing else:
no sign in, no list of worlds, no way into one. It is not a client, it is a
viewer with command line flags. That has to change, and changing it well is
not a question about egui.

**What it changes.** CLAUDE.md says "Svelte (web) and the *minimal* native UI
sit over one command/event seam". This entry proposes dropping the word
minimal for everything a player does, and keeping it for everything an
operator does: sign in with a one time code, list worlds, enter one, preview a
planet with its knobs and create a world from it, and the settings panel that
already exists. The backoffice stays on the web. It is table heavy, it is
rare, it is for the few people who run an instance, and it is the one place
egui would cost far more than it returns.

**Why egui is not the hard part.** Its contract is textured triangles and
scissors, our painter already serves all of it (38), and the design system it
would have to match is monospace, flat, black and white. A list, a code field,
sliders and a picker are what egui is good at. The hard part is elsewhere.

**Where the hard part is.** Today the cold plane lives in the web app's server
routes: SvelteKit calls PocketBase with its SDK, validates, clamps and writes.
None of that is reachable from Rust, which is the real reason the desktop can
do nothing. So the calls move behind the same command and event seam the
engine already sits on: `list_worlds`, `create_world`, `enter_world` as
commands, `worlds_listed` and the existing `rejected` as events, with the
clamping that `+page.server.ts` does now living in the client crate where both
front ends reach it. Then the invariant is finally true rather than aspired
to: one seam, two thin front ends. The engine has worked this way since the
start; nothing else has.

**What it costs, named.** The web app forbids a user visible literal string
and keeps `en.ts` as the source of truth. A Rust front end needs the same
strings, and two sources of truth for them would rot within a month. They are
generated from `en.ts` into a Rust module by a script, the way `proto/` will
generate the wire types for three languages. The second cost is the one to
watch and not to pay twice: every screen a player sees now exists in Svelte
and in egui, so a screen that belongs to neither should be built for neither.

**What this is not.** Not a webview inside the desktop app: it drags a browser
into a binary that also has to run on a Raspberry Pi, a Quest and a Pico
(M6), where there is no webview and a controller instead of a mouse. A native
UI that stays a panel survives that; a native UI that is a whole application
does not. That is the argument for holding the line at player screens.

**How a world is opened, and it is a URL rather than an id.** An id alone
assumes there is one instance, and this is self hostable free software, so
which instance is never optional. The web already addresses a world at
`/w/<id>`, so the desktop takes the same string a person copies out of the
browser: `bun run desktop <url>`, with a bare id meaning the instance it is
configured for. One address works in both, pasting a link becomes the natural
gesture, and it is the same scheme that will carry a place inside a world
(ROADMAP, M1) rather than a second one. It also gives `bun run shot --world`,
which the backoffice and CI both want.

That flag is not a cheap one, and saying so is the point: a recipe lives on the
server, so `--world` is already the thin end of this entry rather than a
sibling of `--seed`. It needs the seam, an HTTP client, and a local cache of
recipes, which permanent offline mode wants anyway.

**An instance's main world is not a desktop concept.** What the front door
opens is a property of the instance, read by the web and the desktop alike, so
it belongs in the cold plane beside `worlds` and not in a native flag.

Rejected: the desktop as a viewer with flags (what we have, and the complaint
that opened this); the desktop as a full peer including the backoffice (two
admin UIs, forever, for the few); a webview shell (no Pi, no headset); two
string tables (rot); a world id with no instance (one host, forever).
