# 103. The palette is the world's: its sixteen colours are kept with what is built (decided)

Logged 2026-10-04.

A cell is an index into sixteen colours (75), and the sixteen are the
world's: kept in the build plugin's store, in the world folder, beside the
cells that name them. A new world starts with the version's palette. A
world's admin may change its colours later.

**Why the world's.** A stored cell says a number, never a colour. With the
palette in the version, a build changes colour the day a version changes its
sixteen, and nobody in that world asked for it. With the palette in the
world, a world shows what its builders saw, through every upgrade, and the
folder that is copied into the next generation (89) carries its colours with
its cells.

**Why it may be changed.** Marlus: it is the world's, and it may be
customised in the future. A world that owns its palette is the one that can
change it, and a change is an op of the build plugin as a stroke is: checked,
logged, taken back (93).

**Rejected:** the palette in the version; a colour in every cell, three bytes
where one says it.

**Lives in:** ROADMAP.md § Building; `client::build::PALETTE`, which is a
new world's palette.
