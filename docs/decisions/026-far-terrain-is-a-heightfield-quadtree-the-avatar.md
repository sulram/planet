# 26. Far terrain is a heightfield quadtree; the avatar is procedural (decided)

Logged 2026-09-20.

Walking a planet needs ground to orbit LOD before it needs editable voxels, so
the first terrain is heightfield patches from the generator. Surface nets
chunks replace only the deepest levels later, and the structure stays. The
avatar is a figure of boxes until the avatar format question in OPEN is
settled. Patches are built on the main thread under a per frame budget, which
is the one approach that runs the same on native and WASM; worker threads are
an optimisation behind the same queue.
