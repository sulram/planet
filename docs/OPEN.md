# OPEN

The only place where unsettled questions live. When one is settled: log it in
DECISIONS.md, update ARCHITECTURE.md, delete it here.

## Blocks M0

- **Name.** `planet` is a codename. Crate, repo and domain names wait on this.
- **Confirm the proposed entries in DECISIONS.md**: 03 (raw wgpu), 05 (numbers),
  06, 09, 19, 20, 46 (the desktop is a whole player).

## Blocks M1 (the address is the save format)

- **Block edge**: 0.5 m like Cryptovoxels, or finer (0.25 m)?
- **Sector resolution**, now two questions rather than one (DECISIONS 48): the
  size the voxel design is **proved** at, and the size a world **ships** at.
  At `2^10` (326 m) the whole level pyramid fits and can be judged; at `2^16`
  it is eight or nine levels deep. Either way `SECTOR_BITS` stops being a
  compile time constant and enters the recipe. The old framing, which assumed
  one size for everything: `2^16` per side (radius about 20.9 km)? Measured at
  `2^19` and reverted (DECISIONS 45). The trade, per bit, with `relief_m` at
  its default: `2^16` is 48 times exaggerated with Chile 590 m wide, `2^17`
  24 times and 1.2 km, `2^18` 12 times and 2.4 km, `2^19` 6 times and 4.7 km.
  `relief_m` buys drama back at any size: 2500 at `2^18` is 22 times with four
  times the ground. What also has to move with the size: the atmosphere's
  density and height (a uniform shell cannot cover both a blue zenith and a
  far horizon; that is the real-scattering item in ROADMAP), the snow line in
  `material`, and three more octaves in the generator to hold ground detail.
- **Build band height**: +-128 m? How deep may people dig?
- **Chunk size**: 16 or 32 per side?
- **Confirm decision 25**: tangent warp as the quad sphere mapping.
- **Relief against the build band**: generator v1 raises peaks to 1400 m while
  the band is about +-128 m. The band now bounds where a volume may sit, so
  the question is whether it follows the terrain surface.
- **What a body collides with inside a GLB shell** (DECISIONS 58). 47 says
  collision is the generator, always, and a shell is a mesh we cannot read.
  Either the volume is the truth and the GLB is a skin over the same cells,
  which keeps one rule and costs a voxelizer at import, or mesh collision
  becomes a capability we build. The first is cheaper.
- **Look**: terrain material style (flat colors, pixel textures, triplanar)?

## Later

- **Default asset set.** `assets/` is committed for now, imported from `refs/`
  by `bun run assets`. Avatars are CC0. The locomotion clips come from Hyperfy
  (GPL-3.0-only, Mixamo rig) and are temporary: replace them with clips from
  Mixamo or our own before choosing a license that GPL does not fit. Where the
  default set is hosted later (repo, LFS, bucket) is open too.
- **Fields: where they are hosted.** 25 MB each, gitignored, baked by
  `bun run field`. Same question as the default asset set, and the same answer
  when it comes. Whether an instance should offer more than one is open too.
- **The simplex kernel steps, and the warp rides on it** (63). `simplex_d`
  uses `0.6 - r²` over four corners, so value and gradient both jump a little
  at every simplex boundary, and `plates::shape` warps its domain by that
  gradient. Worst seen: 21 m of seabed, 250 m under water. `0.5 - r²` is
  continuous and costs amplitude; either way every world and every golden
  changes, so it waits for a reason to spend that.
- **License.** "Free software end to end" names no license yet.
- Account deletion: superuser only today; `worlds.owner` has no cascade.
- World ownership transfer: the update rule allows it with no consent step.
- PocketBase rate limits are off; the code request both mails and creates
  accounts. Enable before any public deploy. No `SMTP_TLS` env yet.
- The magic link is consumed on GET; mail scanners that prefetch could burn
  it. Hardened variant: a confirm button page.

- **How much of a world a signed out desktop may reach.** Offline preview is
  permanent, so a picker of seeds and fields visited before could work with no
  account at all. Whether it should, and whether a world list is worth caching
  for a flight, is open (DECISIONS 46).
- ECS crate: `hecs` or `bevy_ecs` standalone.
- Protocol schema language: protobuf or flatbuffers.
- How much building is possible outside the browser (Pi, headset).
- Cubes on sloped smooth terrain: auto-flatten on build, or leave gaps.
- **How fine per-user rollback has to be.** Per cell attribution costs an owner
  byte in every cell, which doubles a terrain chunk. Rollback at chunk
  granularity is nearly free but takes a neighbour's edits in the same chunk
  with it. With volumes (M4) a chunk usually has one owner, so chunk
  granularity is probably enough, but that is a choice and not a fact. It
  decides what the op log's digest tier has to carry (ROADMAP, M3).
- Scripting model: server-side; language and sandbox undecided.
- Chain for assets and deeds (leaning Tezos); when to anchor snapshots in Bitcoin.
- Opening the map: which region first, how fast.
- Meta store policy on crypto features (only matters if wallets ship on Quest).
- Hyperfy license: confirm GPL-3.0.
- Terms of use for uploaded content.
