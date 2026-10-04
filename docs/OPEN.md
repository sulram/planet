# OPEN

The only place where unsettled questions live: a question with no answer, or a
problem with no chosen fix. Decided work lives in ROADMAP. Settled: the why goes
to DECISIONS, the state to its theme doc, and the row leaves here.

## Waits on Marlus

| Question | Unblocks | Context |
|---|---|---|
| **The name.** `planet` is a codename | crate, repo, image and domain names | README |
| **Confirm the proposed entries** 03 (raw wgpu), 05 (numbers), 06, 09, 19, 20, 25 (tangent warp) | code that depends on them; 25 is the address, and the address is the save format | DECISIONS |
| **Which of entries 51 to 55 survive 58.** They came out of the voxel pyramid 58 retired: the frame budget counting chunks (51), the far field as the same grid coarsened (52), nothing coming down before its replacement is up (53), one level per piece of ground (54), coarse ground not casting (55). Some are still cited by ROADMAP; the rest get their titles struck | a reading pass over the decisions | DECISIONS 58 |
| **A fourth shadow cascade.** Three step tenfold (40 m, 400 m, 4 km): a texel is 7.8 cm near the eye and 78 cm from 35 m on, wider than a cube. Tried and put back: four at 16, 100, 630 and 4000 m, three constants in `render::shadow`. The shadow of an avatar gets its legs and meets its feet; a texel is 3.1 cm near the eye and 20 cm out to 100 m, and 1.2 m from there to 630 m against 78 cm with three. It costs one more pass of casters. The one reading of that cost, 0.2 ms a frame on the development machine, is taken beside a browser running the world and is no measure: measure alone first, and the numbers go in the commit | shadows of builds seen from further than 35 m; the contact shadow of an avatar | ROADMAP wishes · DECISIONS 34, 83 |

## Blocks hosting

| Question | Unblocks | Context |
|---|---|---|
| **Where fields are hosted.** 25 MB each, gitignored, baked by `bun run field` from a public source. A field is immutable and a recipe names it by content id, so it can live outside the image: a public folder on a CDN with an index, read by the founding screen and fetched by the browser. Until fields have that home, an image built in CI carries the generated ground alone | Earth in a published version; a second field | DEPLOY.md · DECISIONS 44, 72 |
| **What a planet says about itself.** mundos's catalog and door show a title, a description and an image that a world's builders set inside it. A planet has nowhere to set them yet, and answers with its name alone | a planet in the catalog with a face | mundos `docs/BRIEFING.md` § Domain |

## Blocks plugins

| Question | Unblocks | Context |
|---|---|---|
| **A plugin's server half: Go, or its own Rust as WASM.** A Go package compiled in is the short path and writes every rule twice, once for the client to predict and once for the server to check. The plugin's Rust compiled to WASM and run by the server through wazero writes it once, and is what 09 proposes for the generator | the first plugin that checks an op: building | DECISIONS 09, 88 |
| **A plugin's panel outside Svelte.** On the web it is a Svelte component. On a desktop or a headset it is written again for that front end, or a plugin says its panel as data (buttons, a palette, a slider) and each front end draws it | the second screen | DECISIONS 88 · ROADMAP § Other screens |
| **May a plugin stand on another.** A wallet's avatars change what the avatars plugin offers: through a hook the core holds, or by importing the avatars plugin | the first plugin that changes another | DECISIONS 88 |
| **A world with a plugin the client lacks.** Refuse to enter, or enter and leave that layer undrawn. Moot on the web, where a world serves its own client | an installed client | DECISIONS 91 |
| **A plugin that needs a shader of its own.** A plugin hands the picture plain data and never touches wgpu. Volumes already have a pipeline of their own in `render`: it stays in the core as a shape any plugin may hand over, or the rule gets an exception | extracting building | DECISIONS 88 · RENDER.md § Volumes |

## Blocks building

The address is the save format: settled before the first volume is stored.

| Question | Unblocks | Context |
|---|---|---|
| **Build band depth**: the band is +-128 m at `2^16`; how deep may a volume dig? A volume holds from the lowest ground of its plot up, and nature is not dug | where a volume may sit | DECISIONS 49, 78 |
| **What a body collides with inside a GLB shell.** 47 says collision is the generator, always, and a shell is a mesh we cannot read. Either the volume is the truth and the GLB a skin over the same cells (one rule, a voxelizer at import), or mesh collision becomes a capability we build. The first is cheaper | the cave: a shell and a room | DECISIONS 47, 58 |
| **Look**: terrain material style: flat colours, pixel textures, triplanar? | the material contract past procedural detail | RENDER.md |
| **How fine per-user rollback has to be.** Per cell attribution costs an owner byte in every cell, doubling a chunk. Chunk granularity is nearly free but takes a neighbour's edits in the same chunk with it; a choice, not a fact | what the op log's digest tier carries | ROADMAP § Building |

## Later

| Question | Unblocks | Context |
|---|---|---|
| **Default asset set: where it is hosted** (repo, LFS, bucket). Committed for now, imported from `refs/` by `bun run assets`; avatars are CC0 | a public release | `assets/` · RENDER.md § Avatars |
| ECS crate: `hecs` or `bevy_ecs` standalone | entities | DECISIONS 03 |
| How much building is possible outside the browser (Pi, headset) | ROADMAP § Other screens | ARCHITECTURE |
| Scripting model: server side; language and sandbox | scripts | VISION |
| Chain for assets and deeds (leaning Tezos); when to anchor snapshots in Bitcoin | wallets | VISION |
| Opening the map: which region first, how fast | land | VISION |
| Meta store policy on crypto features; only matters if wallets ship on Quest | ROADMAP § Other screens | |
| Terms of use for uploaded content | files | |

## Problems with no chosen fix

| Problem | Context |
|---|---|
| **WORLD.md stands at its 200 lines.** The next fact about the world has no room, and which theme leaves for a doc of its own is not chosen: where you are (the place code and the pose), or the recipe and its generators | CLAUDE.md § Docs · `scripts/docs.ts` |
| **The simplex kernel steps, and the warp rides on it.** `simplex_d` uses `0.6 - r²` over four corners, so value and gradient both jump a little at every simplex boundary, and `plates::shape` warps its domain by that gradient. Worst seen: 21 m of seabed, 250 m under water. `0.5 - r²` is continuous and costs amplitude; either way every world and every golden changes, so it waits for a reason to spend that | DECISIONS 63 · `worldgen/tests/cliffs.rs` guards at 60 m |
| **`near` is measured at the reference size.** `near.go` holds `sectorBits = 16`, and a recipe names its own `sector_bits` (49): on a smaller world a line said `near` reaches the wrong distance | `server/internal/world/near.go` |
| **A mute for chat.** Chat limits a line's length and a session's rate. Silencing one person stands on the permission hook, which arrives with the plugin host | DECISIONS 69 · BRIEF.md |
