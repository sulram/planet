# 59. Voxelis is read, not taken (noted)

Logged 2026-09-21.

`voxelis` (crates.io, MIT OR Apache-2.0) is a Sparse Voxel Octree DAG in pure
Rust with hash consed shared nodes, one release in April 2025 and one
maintainer. The licence is the friendliest of any reference we have. We are not
taking it, for four reasons in order of weight.

Its headline is compressing an enormous voxel world, and 58 deleted that
problem: voxels now live only inside bounded volumes. It has no stated WASM
support and depends on `rayon`, which on `wasm32-unknown-unknown` needs
`wasm-bindgen-rayon`, `SharedArrayBuffer` and COOP/COEP headers, so a browser
tab and a Pi would be ours to port. Hash consing shares subtrees across the
world, which has no address and no owner, while our invariants are authority by
address, copy on first write storing a whole chunk, an op log that says who, and
per user rollback. And it would replace a small tested crate to buy what we do
not need.

What is worth reading: its batch edit, which mutates hundreds of thousands of
cells as one operation for a 22 to 224 times speedup. That is exactly the M3
rule that an op is a gesture and not a cell. Its published per operation numbers
on a 32³ chunk are also a data point for the open question of chunk side.

This is the third reference in the same family after Dust (56) and Veloren (57),
and all three point at the lesson 56 already extracted by measuring: a value on
the parent standing for a whole subtree. We have it. It does not need a
dependency.
