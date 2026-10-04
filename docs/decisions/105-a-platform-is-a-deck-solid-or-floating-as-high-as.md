# 105. A platform is a deck, solid or floating, as high as the higher of the ground and the feet (decided)

Logged 2026-10-04.

A platform has one of three bases, picked by the button that lays it: a
deck, the slab on pillars that 78 drew; solid, every column filled down to
the ground (85); floating, the slab alone with nothing under it. Its top is
the higher of two heights: the highest ground under it, and the feet of the
body that asks. At the seam the bases are `deck`, `solid` and `floating`, a
deck when none is said.

**Why the feet again.** 78 laid a slab at the feet first, the hill came
through it, and the top moved over the highest ground. That holds for feet
on the ground, on a slope as on the flat. It fails a body in the air: Marlus
was flying, and the platform was laid far below him, on the terrain. With
the higher of the two, feet on the ground leave it to the ground, as before;
a body over the ground is given the slab where it is; and no slab stands
lower than the ground under it, so no hill comes through.

**What follows from it.** A body standing on a slab is given the next one
level with it wherever the ground under the next is lower: the feet are read
to the nearest cell, so a hair to either side of a cell's edge is the same
slab. A deck laid from the air stands on pillars as tall as the drop, and a
solid one is a block down to the ground.

**Why floating.** Marlus asked for a platform with no columns and nothing
under it. In the air a base is a tower nobody asked for, and a slab that
floats is what a bridge, a balcony or a landing starts from.

**Where it ends.** A volume holds 64 cells, 32 m, over the highest ground of
its plot (77). A platform asked for with the feet over that is refused with a
code, `high`, which a front end says in its own words. How high a volume may
stand is the build band's question (OPEN).

**Rejected:** the feet alone, which 78 tried; a top held at the volume's when
the feet are over it, a slab laid under the one who asked; a base kept as a
setting, which 85 refused.

**Lives in:** WORLD.md § Two layers; GLOSSARY.md; `voxel::platform`;
`client::build`; `client::seam` (`Base`, `BuildRefusal`).
