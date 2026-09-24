# 49. A world has a size, and a grid has a grain (decided)

Logged 2026-09-21.

`SECTOR_BITS` was a compile time constant, so a world could not be small and
the design could only ever be judged at 20.9 km, where no level of anything
fits in one sitting. It is now `sector_bits` in the recipe, frozen per world
exactly as the generator version is, and for the same reason: it decides the
address, and the address is the save format.

**The range is `4..=16`, and both ends are structural.** 16 because `u` and `v`
fill a `u16`, which is the address we already write. 4 because a sector must
hold a chunk: below that the unit of storage no longer tiles the grid it lives
on. A body of `2^4` is 5.09 m of radius and roughly two chunks across, which is
small enough that the 27 chunks around one of them collapse to 15 distinct
ones. That is the point: a world where every part of the design is visible at
once.

**A grid and a body are different things.** Seams, neighbours and the tangent
warp are pure arithmetic that never needed to know what a cell is, so they
belong to a `Grid`: six square faces at one grain. A `QuadSphere` is one body,
a block grid plus the radius that turns cells into metres. The payoff is not
tidiness: the chunk grid is the block grid coarsened by `CHUNK_BITS`, so a
chunk stepping over a seam is a block stepping over a seam, and the property
tests that were written once for blocks now hold for chunks and, later, for
every reduced level. The properties run at every grain from one cell per face
to `2^16`, and two of them were wrong at the coarse end until they did.

**The band is a human measure, bounded by the core.** Half a band is
`min(256 blocks, radius / 4)`. The 256 blocks are a cellar and a tower, which
are the same depth on a 20.9 km body and on a 300 m one, so the band is not a
fraction of anything. The quarter of the radius is the hollow core keeping its
share, and on a body too small to hold a human band the radius wins. At `2^16`
this is exactly the +-128 m the band has always been, so nothing about today's
planet moves.

**The band follows the surface, not the datum.** ARCHITECTURE always said so;
the first implementation here hung it off the datum and a legitimate vertex at
281 m read as a violation, because the relief reaches +-523 m while the band is
128 m thick. A column under a mountain and a column under a trench hold their
chunks at different heights, and `terrain::band_h` is where that is said.

Rejected: a compile time constant behind a feature or an env var, which would
have cost nothing today and made it impossible for one client to hold a planet
and a moon of different sizes, and would have left the size out of the save
format where it belongs; carrying the size inside `Column`, which grows the
address, duplicates one fact per cell, and lets two bodies' integers be mixed
silently; a band that is a fixed fraction of the radius, which gives a 300 m
world a 2 m band and nothing to build in.
