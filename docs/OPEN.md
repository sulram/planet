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
- **Pre-distortion mapping** for the quad sphere: which one (tangent warp,
  Everitt, other)? Needs a small study.
- **Look**: terrain material style (flat colors, pixel textures, triplanar)?

## Later

- ECS crate: `hecs` or `bevy_ecs` standalone.
- Protocol schema language: protobuf or flatbuffers.
- Native minimal UI: egui behind a seam, as in vybe?
- How much building is possible outside the browser (Pi, headset).
- Cubes on sloped smooth terrain: auto-flatten on build, or leave gaps.
- Scripting model: server-side; language and sandbox undecided.
- Avatar format: VRM, custom voxel avatars, both.
- Chain for assets and deeds (leaning Tezos); when to anchor snapshots in Bitcoin.
- Opening the map: which region first, how fast.
- Meta store policy on crypto features (only matters if wallets ship on Quest).
- Hyperfy license: confirm GPL-3.0.
- Terms of use for uploaded content.
