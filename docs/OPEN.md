# OPEN

The only place where unsettled questions live: a question with no answer, or a
problem with no chosen fix. Decided work lives in ROADMAP. Settled: the why goes
to DECISIONS, the state to its theme doc, and the row leaves here.

## Waits on Marlus

| Question | Unblocks | Context |
|---|---|---|
| **The name.** `planet` is a codename | crate, repo and domain names | README |
| **Confirm the proposed entries** 03 (raw wgpu), 05 (numbers), 06, 09, 19, 20, 25 (tangent warp), 46 (the desktop is a whole player) | code that depends on them; 25 is the address, and the address is the save format | DECISIONS |
| **Which of entries 51 to 55 survive 58.** They came out of the voxel pyramid 58 retired: the frame budget counting chunks (51), the far field as the same grid coarsened (52), nothing coming down before its replacement is up (53), one level per piece of ground (54), coarse ground not casting (55). Some are still cited by ROADMAP; the rest get their titles struck | the reading pass at the end of M1.75 | DECISIONS 58 |
| **License.** "Free software end to end" names none yet | the default asset set: the locomotion clips come from Hyperfy (GPL-3.0-only, Mixamo rig) and are replaced, by Mixamo or our own, before a licence GPL does not fit | DECISIONS 01 · `assets/` |

## Blocks volumes (M1.75)

The address is the save format: settled before the first volume is stored.

| Question | Unblocks | Context |
|---|---|---|
| **Build band depth**: the band is +-128 m at `2^16`; how deep may a volume dig? | where a volume may sit | DECISIONS 49 |
| **How a volume seats.** The stamp's flatten (75), a platform of built voxels (a slab on columns down to the ground), or a choice made at opening? And how the platform's size is picked in the mountains, over one or more volumes of the fixed grid | the volume grid rework (ROADMAP M1.75) | DECISIONS 75, 76 |
| **What a body collides with inside a GLB shell.** 47 says collision is the generator, always, and a shell is a mesh we cannot read. Either the volume is the truth and the GLB a skin over the same cells (one rule, a voxelizer at import), or mesh collision becomes a capability we build. The first is cheaper | the cave: a shell and a room | DECISIONS 47, 58 |
| **Look**: terrain material style: flat colours, pixel textures, triplanar? | the material contract past procedural detail | RENDER.md |
| **How fine per-user rollback has to be.** Per cell attribution costs an owner byte in every cell, doubling a chunk. Chunk granularity is nearly free but takes a neighbour's edits in the same chunk with it. With volumes a chunk usually has one owner, so chunk granularity is probably enough; a choice, not a fact | what the op log's digest tier carries | ROADMAP M3 |

## Later

| Question | Unblocks | Context |
|---|---|---|
| **Default asset set: where it is hosted** (repo, LFS, bucket). Committed for now, imported from `refs/` by `bun run assets`; avatars are CC0 | a public release | `assets/` · RENDER.md § Avatars |
| **Fields: where they are hosted.** 25 MB each, gitignored, baked by `bun run field`. Same question, same answer when it comes. Whether an instance offers more than one is open too | a second field (ROADMAP M2) | DECISIONS 44 |
| **How much of a world a signed out desktop may reach.** Offline preview is permanent, so a picker of seeds and fields visited before could work with no account. Whether it should, and whether a world list is worth caching for a flight | the desktop screens (ROADMAP M2) | DECISIONS 46 |
| ECS crate: `hecs` or `bevy_ecs` standalone | entities (ROADMAP M5) | DECISIONS 03 |
| How much building is possible outside the browser (Pi, headset) | ROADMAP M6 | ARCHITECTURE |
| Scripting model: server side; language and sandbox | scripts (ROADMAP wishes) | VISION |
| Chain for assets and deeds (leaning Tezos); when to anchor snapshots in Bitcoin | wallet linking (ROADMAP wishes) | VISION |
| Opening the map: which region first, how fast | land (ROADMAP M4) | VISION |
| Meta store policy on crypto features; only matters if wallets ship on Quest | ROADMAP M6 | |
| Hyperfy license: confirm GPL-3.0 | the license row above | CLAUDE.md § refs |
| Terms of use for uploaded content | uploads (ROADMAP M5) | |
| **Muting in chat.** A mute is a role and roles are per world (M4); until then an operator's only tool is the backoffice | chat moderation | DECISIONS 69 · ROADMAP M4 |

## Problems with no chosen fix

| Problem | Context |
|---|---|
| **The simplex kernel steps, and the warp rides on it.** `simplex_d` uses `0.6 - r²` over four corners, so value and gradient both jump a little at every simplex boundary, and `plates::shape` warps its domain by that gradient. Worst seen: 21 m of seabed, 250 m under water. `0.5 - r²` is continuous and costs amplitude; either way every world and every golden changes, so it waits for a reason to spend that | DECISIONS 63 · `worldgen/tests/cliffs.rs` guards at 60 m |
| The magic link is consumed on GET; a mail scanner that prefetches could burn it. Hardened variant: a confirm button page | ARCHITECTURE § Identity |
| PocketBase rate limits are off, and the code request both mails and creates accounts. On before any public deploy. No `SMTP_TLS` env yet | `server/` |
| Account deletion is superuser only; `worlds.owner` has no cascade | ARCHITECTURE § Identity |
| World ownership transfer: the update rule allows it with no consent step | ARCHITECTURE § Identity |
