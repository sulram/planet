# OPEN

The only place where unsettled questions live. When one is settled: log it in
DECISIONS.md, update ARCHITECTURE.md, delete it here.

## Blocks M0

- **Name.** `planet` is a codename. Crate, repo and domain names wait on this.
- **Confirm the proposed entries in DECISIONS.md**: 03 (raw wgpu), 05 (numbers),
  06, 09, 19, 20.

## Blocks M1 (the address is the save format)

- **Block edge**: 0.5 m like Cryptovoxels, or finer (0.25 m)?
- **Sector resolution**: `2^16` per side (radius about 20.9 km)? Smaller is
  denser and cheaper to fill; larger feels flatter.
- **Build band height**: +-128 m? How deep may people dig?
- **Chunk size**: 16 or 32 per side?
- **Confirm decision 25**: tangent warp as the quad sphere mapping.
- **Relief against the build band**: generator v1 raises peaks to 1400 m while
  the band is about +-128 m. Does the band follow the terrain surface?
- **Look**: terrain material style (flat colors, pixel textures, triplanar)?

## Later

- **Default asset set.** `assets/` is committed for now, imported from `refs/`
  by `bun run assets`. Avatars are CC0. The locomotion clips come from Hyperfy
  (GPL-3.0-only, Mixamo rig) and are temporary: replace them with clips from
  Mixamo or our own before choosing a license that GPL does not fit. Where the
  default set is hosted later (repo, LFS, bucket) is open too.
- **License.** "Free software end to end" names no license yet.
- Account deletion: superuser only today; `worlds.owner` has no cascade.
- World ownership transfer: the update rule allows it with no consent step.
- PocketBase rate limits are off; the code request both mails and creates
  accounts. Enable before any public deploy. No `SMTP_TLS` env yet.
- The magic link is consumed on GET; mail scanners that prefetch could burn
  it. Hardened variant: a confirm button page.

- ECS crate: `hecs` or `bevy_ecs` standalone.
- Protocol schema language: protobuf or flatbuffers.
- How much building is possible outside the browser (Pi, headset).
- Cubes on sloped smooth terrain: auto-flatten on build, or leave gaps.
- Scripting model: server-side; language and sandbox undecided.
- Chain for assets and deeds (leaning Tezos); when to anchor snapshots in Bitcoin.
- Opening the map: which region first, how fast.
- Meta store policy on crypto features (only matters if wallets ship on Quest).
- Hyperfy license: confirm GPL-3.0.
- Terms of use for uploaded content.
