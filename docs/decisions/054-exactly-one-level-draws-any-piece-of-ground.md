# 54. Exactly one level draws any piece of ground (decided)

Logged 2026-09-21.

The shadows were wrong in a way that pointed at the terrain, not at the shadow
code: wide straight bands lying across open ground with nothing above them to
cast one. The terrain had a corduroy ripple on every slope at the same time,
and both were the same fault.

Levels overlapped. A level's shell started inside the reach of the level under
it, by about two and a half chunks, so in that band two surfaces of different
grain were drawn in the same place. They fought for the depth buffer - that was
the ripple - and both cast into the shadow map - that was the bands.

The overlap was on purpose, to be sure of no gap. The right way to be sure of
no gap is not to leave slack in a distance test, it is to cut the levels out of
each other: a chunk whose eight children are all wanted one level finer is
covered, so it is not drawn. That is exact both ways. No overlap, because
covered means covered; no gap, because a chunk is kept unless every piece of
it is taken. It uses the same parent and child mapping 53 needed, so seams
come along free.

The cost is that a level's inner edge moves whenever the level under it does,
so moving 8 m at level 0 reworks every level above. Measured, `bun run bench`:
the worst frame went from 5.91 ms to 8.85 of a 12 ms budget, median 2.36, none
over. That is the honest price and it is paid.

Shadow casters are also now what the pyramid wants and not what is retiring
(53): a chunk held up so the ground has no hole in it would otherwise double
every shadow through the handover.

What is left, and is a different fault: a fine dither on lit slopes, which is
shadow map acne rather than geometry.

Rejected: keeping the overlap and picking a caster per place, which leaves the
depth fight; a fade between levels, which needs the renderer to know what a
level is; widening the shadow cascades, which was never the problem.
