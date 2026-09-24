# 47. Collision reads the field, and a cave is ground (decided)

Logged 2026-09-21.

A heightfield has one ground per column, so a walker on one stands on the roof
of every cave it crosses. With the deepest quadtree level meshed from the
density (44, and the entry before this one in ROADMAP), you could see a cave
mouth and an arch and then walk over both. This entry says what holds a body
up now.

**A footing.** What a body stands on is the top of the solid at or under its
feet, and what is over its head is the bottom of the solid above them. The
surface out in the open, the cave's own floor and roof inside one. It is read
from a column of the density field, probed down from the feet, and refined by
bisection to the millimetre.

**Why not a collision mesh.** ROADMAP asked for a representation of its own,
coarser than the render and built only where someone is, the way Voxel Plugin
builds collision around invokers. Measured in WASM, a footing costs 2 us and a
step, which asks for four, costs 8. There is nothing there to cache: a
collision mesh would hold exactly what `Generator::column` already answers
from, and a cache of something costing microseconds is not an optimisation, it
is a second truth to keep in sync with the first. What will earn a
representation is the edit: the day a stored chunk carries one the field is no
longer the whole truth, and collision reads `chunk(addr)` like every other
reader. The invoker idea comes back then, with something to hold.

**The rules, and each is one number.** A rise of one block is taken in stride
and two is a wall, to be jumped or flown, going up and coming down alike. A
body needs its own height of room to walk into a place, except that a body
already under a low roof keeps whatever room it had, so a tight place is never
a place to be stuck in. Blocked, a step is tried along one address axis and
then the other, which is what slides a body along a wall instead of sticking
it to one.

**Rock at the knee means in, not on.** Standing exactly on the ground and
standing buried in it read the same at the feet, because the density is zero
on the surface. Half a metre up they do not. So a footing asks at knee height:
rock there means the body is inside rock, and then the floor is the ground
itself, which is solid and above. One rule does three jobs: a wall taller than
a footing looks is not walked into, a body that got buried is let out upward,
and nothing falls through the planet however fast it arrived.

**Probes are half a cell.** A cell of the volume is one block, and a slab one
cell thick is drawn, so a probe as coarse as a cell straddles it and a body
falls through a ledge it can see. The test that found this walked eight
directions out of a cave mouth and came down through a half metre lip.

**Collision is the ground in full detail**, never the filtered one (29): what a
body stands on may not change with how far away the camera is. The drawn mesh
is band limited and the two differ by less than a cell, which is the same
difference the heightfield walker already lived with.

**The camera goes into the cave.** A third person boom was kept over the
terrain as drawn, so that nobody looks at the ground from underneath. Inside a
cave that rule points the wrong way and would yank the camera out through the
roof, so there the boom is cut by the rock behind it instead: the camera stays
in the cave, as near the body as the walls allow.

**What this does not fix, and it shows.** Collision reaches as deep as the
field does; the volume is drawn about 8 m under the lowest ground of a patch,
which is what a frame affords. A shaft deeper than that drops a body out of
what is drawn, into a world that is hollow under its shell. The fix is the job
queue M1.5 already asks for, not a change here, and until it lands the honest
statement is that a cave is walked from its mouth.

Rejected: a collision mesh built around invokers (nothing to cache until edits
exist); collision against the drawn mesh (a client's LOD would decide where
the ground is, and the server is authoritative by address); holding a body at
the depth the volume happened to be drawn to (the ground would then depend on
what a client had loaded, which is the same fault in a smaller coat).
