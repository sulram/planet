# 77. Volumes are cut by the address: a fixed grid of plots, and neighbours are read as one (decided)

Logged 2026-09-30.

A sector's columns are cut into plots of 64 blocks a side, and the plot of a
column is its `u` and `v` less six bits. A volume stands over one plot, as
tall as it is wide, and taking a tool where none stands opens the volume of
the plot under the body. Volumes touch. A cell is the address it has, so a
stroke, a line of sight, the sides that show, a footing and an undo read the
volumes of a sector as one, and a build too big for one volume stands over
two neighbours. A volume opened beside another, by a side or a corner, takes
its floor. A plot on the edge of a sector stays nature.

**Why a grid.** Marlus' direction after the first cut. 76 opened a volume
around the body and kept 32 blocks between two of them for their margins, so
a build that reached the edge of its volume had nowhere to go, and where a
person happened to stand decided the shape of the land for good. Cut by the
address, two volumes cannot overlap and need no hook to say so, a volume is
named by where it is, and a plot is a column of the block grid coarsened by
six bits, which is how `topology` already names a chunk (49).

**Why 64.** It is the side 76 opened, 32 m at the middle of a sector: room
for a house. A power of two makes the plot a shift of the address, and four
chunks to a side means a chunk never straddles two volumes.

**Why neighbours are read as one.** A wall that crosses from one volume to
the next is one wall: the side between two cubes is hidden, a corner is shut
in by the cube next door, a drag does not stop at a line nobody sees, and a
body walks over it. So the volumes of a sector share one frame, where a cell
is its address, and `voxel::Volumes` answers for all of them. Each volume
keeps its own cells and its own floor, and what reads cells is written once
over whatever holds them. A gesture takes of each volume what it holds, and
off every plot there is nothing to take.

**Why a neighbour takes the floor.** A build across two volumes wants one
floor. The ground under the feet, which is what 76 takes, is a slope beside a
volume: its own margin. So the first volume of a platform takes the ground
under the feet, and each one opened against it takes its floor, the nearest
to the body where it touches more than one. Two platforms that grow until
they touch meet as terraces: the higher floor is a wall to the lower volume,
and a stamp holds its footprint under the margin of any other, where before
the stamp laid last bent the floor beside it. This is 75's flatten carried
over a grid and nothing more: how a volume seats is in OPEN.

**Why the edge of a sector stays nature.** A stamp holds within its sector
(75) and the corners of a sector are zoned as nature (WORLD.md). A build
across a seam needs the stamp and the cells folded over it, which `topology`
can do and nothing asks for yet: it is in ROADMAP.

**Rejected:** a volume around the body, and a margin between two; a floor of
its own for every volume, which steps the floor of every build that crosses;
refusing the volume between two platforms, which leaves a hole nobody can
fill; every volume alone, stitched by the client, which writes the seam
between two volumes once per front end; a public trait for what holds cells,
where a private one serves (65); one store of chunks for a sector, cut by the
address along `h` too, which is where the save format points and waits for
the first stored chunk (OPEN).

**Lives in:** WORLD.md § Two layers, `crates/voxel/src/volumes.rs`,
`crates/client/src/build.rs`, `crates/worldgen/src/lib.rs`.
