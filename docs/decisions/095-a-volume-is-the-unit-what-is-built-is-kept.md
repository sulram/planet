# 95. A volume is the unit: what is built is kept, permitted and rolled back by volume (decided)

Logged 2026-10-04.

A volume, the cells over one plot, is the unit of three things. The build
plugin keeps cells and their history by volume. Permission to build is given
by volume. Taking back what was done wrong rolls a volume back to an earlier
moment, whoever built in it since. A person's own undo stays what it is: it
takes back their strokes.

**Why one unit.** Marlus's rule: what is undone is the area that is permitted.
A place given to someone to build in is the place answered for, so the box a
permission names is the box a rollback restores. Rollback by person asked for
an owner in every cell, doubling a chunk, or took a neighbour's work with it
chunk by chunk, along an edge nobody sees. A volume's edge is the plot's, and
a builder knows it: it is where their permission starts and ends.

**Why sharing it costs nothing.** A plot is the address less six bits (77),
the core's word. The build plugin files cells under it and land files
permissions under it, and neither names the other (93).

**What is kept.** A cell stays one byte, a paint. The op log says who, by
account, so an audit names the builder with no owner in a cell. How far back a
volume rolls is how many versions of its chunks are kept, the knob the undo
window already turns.

**Rejected:** an owner byte in every cell; rollback by chunk, boxes of 8 m cut
across a build; rollback by person past the undo of their own strokes.

**Lives in:** WORLD.md § The world is a recipe; PLUGINS.md § The core's words;
ROADMAP.md § Building.
