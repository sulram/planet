# 82. Volumes cast the sides the sun reaches, as the ground does (decided)

Logged 2026-09-30.

Volumes stay in the shadow pass of the ground: the sides turned to the sun
are drawn, with its slope bias. Casting their far sides with no bias, which
80 left as the next step, is measured and not taken.

**Why.** Far sides are the answer to a lit side that shadows itself, and
after 80 no lit side of a volume does. At the place of the report a wall
stands on a platform with the sun grazing the side it lights, at clocks 100
and 120 of the day, bloom and clouds off. The same frame with shadows on and
with shadows off differs on that side in 263 of 36,000 pixels at the first
clock and 38 at the second, all of them along its rim. What the map does to a
lit side of a volume is nothing to mend.

Far sides also have a cost the measure did not need to pay to see. The caster
moves a cell further from the sun, half a metre, and the offset that lifts a
receiver off its own surface is about half a metre in the second cascade:
ground close under a slab would be lifted past the slab's underside and lit,
where today the top of the slab still shades it.

This is 55 again, on cubes: a technique a shipped voxel game uses, that reads
as principled, tried against a frame and found to change nothing.

**Rejected:** far sides for volumes alone, a second shadow pipeline that
mends nothing; a ceiling on the slope bias, for the same reason.

**Lives in:** RENDER.md § Sea, sky and light, `crates/render/src/lib.rs`
(`pipeline`).
