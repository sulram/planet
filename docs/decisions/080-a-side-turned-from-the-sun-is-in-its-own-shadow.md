# 80. A side turned from the sun is in its own shadow, and only a side the sun reaches asks the map (decided)

Logged 2026-09-30.

A side whose normal turns from the sun takes no sun and the fill of a cast
shadow, by its normal alone. The shadow map is asked only by a side the sun
reaches, and what it answers comes in across a terminator 0.1 wide in the
cosine of the angle.

**Why.** Marlus built a wall on a platform and its shaded side came out in
pale blocks, stepped along the top and down the corner. That side was turned
from the sun, so it had no sunlight to lose. What the blocks changed was the
fill, which a cast shadow dims to 0.45 so that relief does not read as a tint.
Every lit surface asked the map, whichever way it faced, and about a side
turned from the sun the map has nothing to say: what it holds there is the far
side of the same wall, the sample is pushed 1.3 texels off the face, and near
the silhouette that lands outside the wall's shadow, further the more the sun
grazes the wall. The steps were the texels of the first cascade, 7.8 cm each.
The side of a platform, a pillar and the back of an avatar showed the same.

A map with no error would say shadow for every such side. The normal says it
with none, and those sides take no samples at all.

**What changes beside the fault.** Ground turned from a low sun was lit in
the map, since ground seen from under it is culled from the shadow pass, and
it is in shadow now like any other side turned away. Measured at the place of
the report over a dawn, the meadow is 13 to 20% darker on screen while the sun
comes up, 6% when it stands a little higher, and the same from there on: a
side the sun meets at more than 6 degrees is untouched. The top of a platform
and the shaded side of a wall keep their level and lose their blocks. By day
only the sides that showed the fault change.

**Not done.** Volumes cast through the pipeline of the ground, with a slope
bias that has no ceiling, where a closed solid could cast its far sides with
no bias at all. The kernel is four taps. The cascades step tenfold, so a cube
35 m from the eye is smaller than a texel of the cascade that holds it. All
three are in ROADMAP.

**Rejected:** a larger bias or offset, which moves the leak and lifts shadows
off what casts them (55); fill that does not look at shadows, since the
dimming is there so that relief does not read as a tint; a wider terminator,
which takes light from flat ground under a low sun; the rule inside the map
lookup itself, where the sea asks with the local up and would lose the sun
on it as the sun goes down.

**Lives in:** RENDER.md § Sea, sky and light,
`crates/render/src/shaders/common.wgsl` (`lit_occluded`, `TERMINATOR`).
