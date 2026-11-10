---
date: 2026-11-10
lives_in:
  - LAYOUT.md#crates
---

# One GLB reader, in a crate of its own

**Decision:** Opening a binary glTF, walking its nodes at rest and reading the meshes its scene places, their materials and the sizes of their pictures live in the crate `glb`, the one GLB reader. `avatar` reads a skin and a clip through it, and the client a 3D model. `glb` imports no crate of ours; `avatar` and `client` import it. It holds every index of a file to name something before a reader walks it, since a 3D model is any builder's upload and is read by every visitor's engine.

**Why:** A 3D model and an avatar are both GLBs, and two parsers would be two to keep safe against a hostile file. A model is no avatar: read through `avatar`, that crate would speak of materials and scenes that are not its nouns.

**Rejected:** The static reader inside `avatar`, and the client importing it for models: `avatar` would name what is not its own · a second reader in the client: two parsers to hold against the same hostile files.
