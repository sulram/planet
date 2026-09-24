# ~~48. The world is voxels to the core, and the heightfield goes~~ (retired by 58)

Logged 2026-09-21.

The ground has been a heightfield since the first planet: a quadtree of
patches, sampled from `sample_at`, beautiful from orbit and cheap on a
Raspberry Pi. Entry 47 and the two before it bolted a volume onto its deepest
level so that a cave could exist. This entry proposes finishing the job the
other way round: the world is voxels, and a height is at most a way of
summarising them from far away.

**What the hybrid actually is.** A heightfield is a surface of no thickness.
The volume layer is a 16 m window of density meshed where the quadtree runs
out of levels; inside it the world is solid, and one metre below it there is
not rock, there is nothing. Walk into a shaft and you come out under the
world and see the sky through the ground. Every attempt to fix that was the
window again: deeper, then a ring of patches around the body, then a cap on
the chunks one patch may mesh, each one a constant tuned against a bench. That
is the signal a design is wrong rather than unfinished, and the work is parked
on `feat/voxel-volume` instead of continued.

**It is not a culling fault.** In a voxel world a face exists only between
solid and air, so a closed world cannot be seen from the wrong side. What
looked like backface culling was the absence of an interior.

**Orbit is a requirement, not a later concern.** A world has a planet, a moon
and whatever people put in orbit, and all of them are voxels. So the question
is not whether voxels can be near the player, which is easy, but whether a
body made of them can be drawn whole from outside. The arithmetic says yes and
says what it costs: a body of `2^n` blocks a sector side has
`6 * (2^n / 16)^2` surface chunks, which for today's 20.9 km planet is 100
million, and a pyramid of reduced chunks reaches a drawable count in eight or
nine levels, one cell of the coarsest being 128 m. That is the same depth the
heightfield quadtree already runs at. The difference in cost is that a
heightfield level is sampled and a reduced chunk is built, and it has to keep
the mean or a hill grows and shrinks as the camera moves (43).

**The rule the hybrid got wrong, stated properly.** A far level may be hollow,
because nobody can be inside it. The near field must have an inside, as deep
as anybody can go, which is the build band and not a window chosen against a
frame budget. Then every level boundary is volume to volume, and nothing hands
over from a solid to a surface.

**Smaller worlds stop being a nicety.** `SECTOR_BITS` is a compile time
constant, so every world is 20.9 km. It becomes part of the recipe, frozen per
world like the generator version. That is what makes the design provable: at
`2^10`, a radius of 326 m, every level fits and the whole pyramid can be
judged before it is asked to hold a planet. It is also the first real piece of
work, because it touches the address, the generator, the client and the wire
shape.

**What survives, and it is most of it.** `topology` is already the article's
design and arrived there on its own: quad sphere with a pre-distorted mapping
(25), addresses, neighbours across seams and corners, property tested.
`voxel` has the chunk blob and surface nets. `worldgen` has deterministic
density with caves and the band limiting discipline. `render`, `avatar`,
`scene`, the command and event seam, the web app and the server do not care
whether the ground is a height or a volume. What goes is `client::terrain`,
`client::volume`, and every rule that exists because the ground was a height.

**The reference.** `https://bowerbyte.com/posts/blocky-planet/`, which is
where this project's shape came from and which solves the same geometry for a
planet small enough to need no LOD at all. Its shells, which keep a block's
width roughly constant with depth by quadrupling the blocks per layer, are the
one part of its design we named in the glossary and never built.

The brief is docs/VOXEL_BRIEF.md. What it deliberately does not settle is in
OPEN.md: the size the design is proved at and the size a world ships at, how
far orbit has to reach before a level may be a shell, when shells arrive, and
whether a visitor first sees blocks or a smooth surface.

Rejected: keeping the hybrid and widening the window again (the complaint that
opened this, and four tuned constants deep); dropping voxels and going back to
the smooth planet that worked (VISION says a world made of voxels, and
building and digging are the point, not decoration); holding a body at the
depth a client happened to mesh, which makes the ground depend on what was
loaded and is the same fault as 47 rejected in a smaller coat; a voxel near
field with a heightfield far field kept as the permanent answer, which is what
is being abandoned and would only postpone the same seam.
