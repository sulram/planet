# 55. Ground coarser than a shadow texel does not cast (decided)

Logged 2026-09-21.

The planet from orbit was covered in huge black triangles. Not acne: whole
triangles of the coarse levels, shadowing themselves completely.

A chunk of level 10 has cells 512 m across. The furthest shadow cascade has a
texel of about 7.8 m. One such triangle spans hundreds of texels and its depth
varies across a single one by more than any bias can lift, so every one of them
fails its own depth test. Coarse ground cannot be in a shadow map at all.

`scene::SHADOW_CASTER_CELL_M` is the line: ground with cells wider than that
does not cast. Nothing is lost, because a body seen from that far is lit by the
angle of the sun, which is what lights a planet from space anyway.

**What this is not.** There is a fine speckle left on lit slopes near the
ground, and it is not shadow bias. Measured against the same frame rendered
with shadows off, the speckle the shadow pass adds is 14.7 with the depth bias
we have and 14.99 with Veloren's technique of casting only the faces turned
away from the sun and no bias at all. Front face casting was tried because a
shipped Rust voxel game does exactly that; on our ground it changed nothing and
was reverted rather than kept for looking principled. Only a bias eight times
larger removes the speckle, and that one detaches shadows from what casts them.

So it is not the shadow. It is small scale geometry shadowing itself, which
means the surface nets vertices are rough at sub-cell scale, most likely from
the density byte saturating past `DENSITY_REACH` on steep slopes. That is a
`voxel` question and it is open, in OPEN.md and nowhere else.

Rejected: a bias large enough to hide it, which trades a speckle for shadows
that float off their hills; widening the cascades, which was never the problem
and costs texels everywhere.
