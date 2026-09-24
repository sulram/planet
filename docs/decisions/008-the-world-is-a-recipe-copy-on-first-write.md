# 08. The world is a recipe; copy on first write (decided)

Logged 2026-09-20.

A planet this size has about 25 billion block columns and cannot be baked. The
database stores only modified chunks; everything else is regenerated from the
seed by every machine. Consequence: the generator version is frozen per world,
or untouched terrain would shift next to existing builds.
