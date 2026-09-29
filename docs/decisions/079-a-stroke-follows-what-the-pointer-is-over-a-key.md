# 79. A stroke follows what the pointer is over, a key turns it, and a platform is laid only when asked for (decided)

Logged 2026-09-30.

The end of a stroke is the cell the pointer is over, read as its start was,
whenever that cell lies in the layer of the stroke. Over anything else the
pointer is read where its line of sight crosses the layer. A stroke lies on
the side it started on, and follows the pointer onto another surface its
start lies on too, as from the foot of a wall up the wall. Alt turns it, each
time it goes down, to the next of the three layers through its start: on its
side, then standing the way the eye sees most squarely, then the other.
Taking a tool builds nothing: a platform is laid by its button alone.

**Why.** Marlus built with 76 and 78 and found three faults. A stroke from
the side of a pillar was always a sheet against it, because its layer was
fixed by the side it started on. Alt chose between the other two layers by
the angle of the eye, which from in front of a pillar is a tie, so the stroke
went one way when he wanted the other. And Alt sometimes did nothing. He
asked how Cryptovoxels did it.

**What Cryptovoxels does** (`refs/retro`, read for ideas). The end of a drag
is read off the mesh under the pointer, the way the start is, and the box is
shown when the two share a coordinate: the layer follows from what is pointed
at, with no key. What is held with the pointer is read off the pointer's own
event. Its limit is the air: a drag ends only on something built, so a wall
rises a row at a time.

**What is taken from it.** The end read off what is under the pointer, and
what is held read off the pointer. Dragging up a wall from its foot clads the
wall, where a layer held to the floor went on behind it. What 76 had is kept
for the air: over nothing built the stroke holds its layer, and its ghost is
exactly the cells it changes.

**Why a stroke follows only a surface its start lies on.** A box wherever
start and end share any coordinate turns a slab into a wall the moment the
pointer passes over a cube in line with its start. A surface that holds the
start too is one the hand came from.

**Why a turn and not a hold.** Two layers stand, and a key held says one
thing. Each press turns once, so Alt held as a stroke starts still gives the
wall 76 gave, and one press more gives the other. A stroke a hand turned
follows nothing: the hand said which layer.

**Why Alt did nothing.** Keys reach the canvas while it has the focus, and a
click on the panel hands the focus to a button. Now the pointer says whether
Alt is held, at each press and move, and while building the canvas takes the
focus back when the pointer is over it, unless something is being typed.

**Why a tool builds nothing.** Taking a tool on a new plot laid a platform
nobody asked for. A platform is a thing built, and what is built is asked
for.

This takes the layer of a stroke and its key from 76, and the first platform
from 78. The rest of both stands.

**Rejected:** Cryptovoxels' rule whole, with no stroke into the air; a box
wherever start and end share a coordinate; a choice of layer in the panel; a
key for each layer; the layer read from the first movement (76); opening a
volume, or laying a platform, by taking a tool.

**Lives in:** ARCHITECTURE.md § Clients and UI, `crates/client/src/build.rs`,
`crates/shell-web/src/web.rs`.
