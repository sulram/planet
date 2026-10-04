# 88. The core is the sphere, and what is done on it is a plugin composed at build (decided; the order proposed)

Logged 2026-10-04.

The core of planet is the navigable sphere and the host of its plugins.
Everything that can be a plugin is one, and where the core keeps a thing it
keeps a hook for a plugin to change it. Plugins are ours, compiled in: a
config lists them, the build bundles them, and a version is the core plus the
plugins chosen for it.

**Why.** Building is where the project changed its mind most (48, 58, 76 to
79), and it is written into `client`: `client::build` meets the rest of the
crate in some thirty five places, so another way of building is a rewrite of
the client. What is done on the sphere is where the experiments are: a way of
building, a rule for who wears which avatar, land, a wallet. As plugins they
are switched on and off, two of them stand side by side as proofs of concept,
and the core stays small enough to be wrapped by another app: a headset, a
front end all in Rust.

**What the core is.** What a plugin stands on, and nothing a world can be
walked without. The body: topology, the generator, the recipe. The picture:
the renderer, the sky, the sea, the ground. A body walking it: the controller,
the footing on the ground, the camera. The link: the socket, the session, its
level (87), a stance for each session. And the host: the registry of plugins,
the hooks, the command and event seam, a store for each plugin, the files.

**What a plugin is.** A slice through up to four places: a crate in the
client, a package in the server, a payload on the wire inside the core's
envelope, and a panel in each front end. The Hyperfy fork has this in one
language, a folder with `client.js` and `server.js` listed in
`hyperfy.config.js`; here the slice crosses Rust, Go and Svelte, which is the
cost, and the reason the host is cut by extracting what exists and never drawn
ahead of it (65).

**Native plugins, in this order.** Chat first: the smallest slice that crosses
the server, the wire, the client and a panel, and nothing of the renderer.
Then avatars: the core draws a figure for a body and the plugin says which.
Then building: the volumes of 58 and 75 to 86 are its design, and its
extraction is what cuts the seams for the frame, the footing and the picture.
A second way of building, a proof of concept, is the second implementation
those seams need to be real.

**Hooks arrive with whoever asks.** Who may do what, and where, is a hook
whose default is the level; land with landlords (14) is a plugin over it.
Which avatars are offered is a hook; a wallet's avatars are a plugin over it.
A hook is written when a plugin asks for it.

**The renderer stays whole.** A plugin hands the picture plain data through
`scene`, as the client does; wgpu stays the renderer's.

**Rejected:** plugins loaded at run time, since a WASM module in a browser
links nothing after it is built and nobody outside writes one yet; a script
sandbox now, which stays a wish; an API for plugins drawn on paper before
anything is extracted (65); cargo features through `client`, the `cfg` sprawl
the invariants forbid; chat and avatars kept in the core because every world
wants them, which is what a native plugin is.

**Lives in:** ARCHITECTURE.md § The core and its plugins; BRIEF.md; CLAUDE.md
§ How it grows; `docs/plugins.md` in the fork for the model.
