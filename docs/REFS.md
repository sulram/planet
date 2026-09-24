# REFS

Reference projects under `refs/`: read-only, gitignored, shallow.
`its-plataforma` and `vybe` are symlinks to sibling working copies. Read for
architecture; the licence decides what may be taken. Anything learned that
shapes a choice goes to DECISIONS.

## MIT or Apache-2.0: code may be copied, with its notice

| Ref | Read for |
|---|---|
| `myth` | shadow atlas, SSA render graph (DECISIONS 61) |
| `bevy` | the largest live Rust wgpu codebase |
| `cesium` | planet scale f64 as camera relative f32, quadtree LOD, skirts against cracks |
| `playcanvas` | clustered lighting on WebGL2 |
| `threejs` | API ergonomics, GLTF |
| `godot` | M3 gizmos and editor UX |
| `valence` | server authoritative voxel protocol |
| `three-vrm`, `vrm-specification` | VRM, for `avatar` |
| `fast-surface-nets-rs` | surface nets |
| `vircadia-world` | entity state in Postgres; unmoved since January 2026 |

## Copyleft or source available: ideas only

| Ref | Licence | Read for |
|---|---|---|
| `retro` (Cryptovoxels) | BSL 1.1, not open source | voxels, in-world building, land |
| `hyperfy` | GPL-3.0-only | GLB entities, scripts |
| `veloren` | GPL-3.0-or-later | far field, voxel reach (DECISIONS 57) |
| `luanti` | LGPL-2.1+ | mapblock streaming, server authority, per world privileges |
| `dust` | MPL-2.0, file level copyleft | DECISIONS 56 |

## Models, not refs

- `its-plataforma`: web app, auth, design system, scripts and docs rules. Its
  `staging` branch is the current one.
- `vybe`: wgpu and winit pins, so knowledge transfers.
