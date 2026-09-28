# 75. A volume is half metre cells in chunks of 16, bent onto its body, on ground a stamp holds flat (decided)

Logged 2026-09-28.

A cell of the build layer is a block of the address, 0.5 m at a sector's
middle, and a volume stores its cells in chunks of 16 a side. A cell is air
or a paint, an index into a palette. Every side of a solid cell that looks at
air is one quad, never merged with its neighbours, and every corner goes
through the address it is, so a volume curves with its body. Three gestures
change cells: create fills air with a paint, delete empties, paint repaints
what is solid and leaves air as air. Under a volume a stamp holds the ground
at its floor and eases it back over a margin, and the ground it holds is a
material of its own, worked earth where nothing grows.

**Why these two numbers, now.** The address is the save format, so the block
edge and the chunk side had to be settled before the first volume was drawn
(OPEN). Half a metre is Cryptovoxels' block (`refs/retro`, read for ideas),
what a person builds a room from; a quarter metre quarters the cells of
every room for detail an entity gives better (M5). Sixteen is what WORLD.md,
`topology`'s properties and the retired chunk blob already spoke, and 4096
cells is what a stroke redraws in a fraction of a frame.

**Why the sides are not merged.** The brief asked for greedy meshing, and it
is wrong on a sphere: a merged quad is a chord, and the small sides beside it
meet it in the middle of an edge, where the surface cracks. Seating a volume
as one rigid frame at its middle would avoid that and put the cubes off the
cells a body walks on: on a world of `2^10` a 64 cell volume sags 0.39 m at
its edge. So every corner is bent through the address map, and a chunk works
out the directions of its column corners once, 289 of them, after which a
corner costs a multiply. Cryptovoxels draws its opaque cubes the same way,
one quad a face.

**Why paint leaves air alone.** In Cryptovoxels paint is create aimed at a
solid cell and fills every cell of its box, so painting across a wall fills
its windows. Here each gesture takes only what it can: create takes air,
delete and paint take what is solid, and create never repaints.

**Why a stamp, and why its own ground.** Cubes on a slope either sink on one
side and float on the other or need the ground under them changed, and nature
is not an edit (58). The stamp changes what the generator gives: it is read
where `sample_at` is, at every footprint, so the heightfield, collision and
spawning agree, and a sample far away pays one dot product for it. Grass is
up to a metre tall and would stand through a cube, so the ground a stamp
holds is `Material::Plot`, which grows none, and shows where building is.

**Rejected:** 0.25 m cells; chunks of 32, eight times the cells redrawn per
stroke; greedy meshing; a volume seated as one rigid frame; a cell of 16 bits
with texture, tint and a spare flag as Cryptovoxels has, where a palette
index says what is needed; leaving cubes on a slope as they fall.

**Lives in:** WORLD.md § Two layers, `crates/voxel`, `crates/worldgen/src/stamp.rs`.
