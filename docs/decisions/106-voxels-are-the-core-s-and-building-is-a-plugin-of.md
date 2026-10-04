# 106. Voxels are the core's, and building is a plugin of how they are made (decided)

Logged 2026-10-04.

The cells of a world are the core's: the volumes on their plots, what is
kept of them, their picture, the footing on them, and the one op that
changes them, a gesture over a box. Building is a plugin over that: its
tools, its kinds of construction, its modifiers, the keys it asks for and
its panel. Another building plugin uses the same cells with another way of
making them.

**Why.** Marlus: voxel stays part of the core, and what the plugin does is
construction, our choices in the manner of Cryptovoxels: the commands in the
browser, the kinds of construction, the modifiers, the keys, Alt among them.
The cells can be made better, or a new building system written over them,
and the experience changes whole with a plugin.

**Where the earlier cut fails.** 88 and 99 gave the grid of cells to
building's plugin. A plugin that is off is unmounted, so a world with
building off would show nothing of what was built in it and let a body fall
through, and two ways of building side by side would hold two sets of cells
that never meet. With the cells in the core, off means nobody holds a tool,
and what stands, stands. Land gives permission by volume (95) to whatever
plugin a stroke comes from, and an agent sends the gesture itself (94).

**What the core holds.** `voxel`, the cells and the gesture; the volumes
seated on the sphere, their meshes and their pipeline in `render`; the
footing on cells and the grass under them; the palette (103); and on the
server the op that applies a gesture, kept, permitted and rolled back by
volume (95), written once in Rust and run in the module (97). The core is an
owner as a plugin is (93).

**What the plugin holds.** How a hand arrives at gestures: a tool in hand, a
stroke from a pointer, a platform as a deck, solid or floating (105), a key
that turns a stroke, the chord that takes one back, the panel. A kind of
construction is the gestures it comes to, so `voxel::platform` is the
plugin's.

**What it settles.** How a plugin draws (OPEN): through the core's shapes,
and the volumes' pipeline is the core's own. A plugin asks for its keys by
name, as data, and each shell binds them.

**What it costs.** The core is the sphere and its cells, and larger for it.
A way of building that is no cell, a mesh or a ground that is sculpted, is a
new layer of the core or a plugin with a state of its own: this cut does not
carry it. The gesture is what every building plugin and every agent stands
on, so a new shape of cell is a change to the core.

**Rejected:** the grid of cells in building's plugin (88, 99); a plugin of
cells that building's plugin imports, since a plugin imports the core and
never another (93); a pipeline for volumes brought by a plugin, a shader of
its own.

**Lives in:** PLUGINS.md § The core's words; ARCHITECTURE.md § The core and
its plugins; BRIEF.md; GLOSSARY.md; OPEN.md.
