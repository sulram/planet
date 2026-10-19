# 111. Parts are a system of the core: a placed thing with a pose, rigid in the frame of its seat (decided)

Logged 2026-10-19.

A part is a placed thing that is no cell: a cube first, then a model, a
picture, a light. The parts are a system of the core beside the cells, kept
by the seat they stand on and never by a volume, so a part stands over a
volume, between two or on nature alike. A part has a pose: an anchor, which
is an address of its seat with fractional blocks and a height over the
datum, a rotation in the flat frame of that point, and a scale in blocks. It
is rigid in that frame. A part is moved in the world, and its pose is read
back from where it came to, so one crossing from a sector to the next is the
same part at the same place with nothing folded. Building's plugin holds the
hand that places it; `plugins/build` is where the one gizmo, keys and panel
live, for every shape of part.

**Why the core's.** Marlus: a cube to add, turn, scale, make solid or not and
paint as the voxels are, the way Cryptovoxels features work. By 108's two
tests it is the core's: with building off, a cube with collision still stands
and still holds a body up; and models of files, pictures and vehicles are
placed things too, so several plugins touch it. The three sessions building
cubes, `.vox` models and images agreed on one record and one gizmo, and each
file kind is a shape of the same part.

**Why not inside a volume.** Marlus: a part must sit between one volume and
another and move from one to the other with no seam. A volume is one plot
and a part is not cut by plots, so a part kept by a volume would be two
parts over a plot line. Kept by the seat, it is one record; permission is
asked for every plot its box covers, through the hook the cells use.

**Why rigid, and read back from the world.** The cells bend each corner onto
the body on its own, so a cube of cells shrinks with the grid toward a
sector's corners. A part is a mesh with one matrix, and a pose must survive a
seam exactly: a rotation expressed in an orthonormal frame is the same
rotation re-expressed in the next sector's frame, which no anisotropic frame
allows. So the frame of a point of a seat is the sector's `u` made unit, up,
and their cross, a block long each (`seat::frame`), and a cube of scale 1 is
a cell at the middle of a sector and a little wider than one at its corners.
`topology`'s `wrapped` preserves address distance, not the world point, so an
out-of-range pose has no honest place: a part's pose is always in range, every
move is a world point, and `seat::reseat` reads sector, address, height, turn
and scale back from the world transform. A property test seats a part on any
sector, reads it back and finds the same pose; another pushes it over an edge
and finds it in range on the next sector where it was pushed to.

**One draw path.** A part is a model: a mesh of a volume's vertices in its own
space, placed by one matrix a frame, relative to the camera as an avatar is.
A cube is a model of one voxel and a `.vox` file a model of many, so both are
lit, edged and glazed by the cells' own shader, whose sides are one file
(`cubes.wgsl`) with a vertex stage each for a volume and a model.

**Footing.** A solid part answers a body as the cells do: the column of the
body, as wide as the body, against the part's box; solid at the height of the
step is a wall, below it a floor, above it a roof. A builder with a tool in
hand flies through parts as through cells.

**Rejected:** a part as cells written into a volume, which cannot turn or
scale; a plugin of parts other plugins import (93); a frame that matches the
grid's shrinking blocks, which no rotation survives across a seam; folding a
pose over a seam with `wrapped`, which keeps the address distance and loses
the place; one gizmo and panel per shape, written three times.

**Lives in:** PLUGINS.md § The core's words; ARCHITECTURE.md § The core and
its plugins; GLOSSARY.md; ROADMAP.md § The parts; `crates/part`,
`crates/seat/src/frame.rs`, `crates/client/src/parts.rs`,
`crates/render/src/models.rs`, `plugins/build/client/src/place.rs`.
