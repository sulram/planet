# 63. A plate margin is a slope, and a wall is a branch taken too late (decided)

Logged 2026-09-23.

Generated worlds grew vertical walls: a razor ridge running dead straight out
of the sea for kilometres, green on a knife edge and a sheer face under it.
Seen end on from a distance the same thing read as a comb of fins along a
ridge, which is why it looked at first like a hole in the terrain mesh. It was
not. Sampling the height across one gave 285.38 m at one point and -476.78 m
a quarter of a metre further on, with both sides smooth: the generator was
asking for the wall and the renderer was drawing exactly what it was asked.

The straightness was the tell. A straight line on a sphere of plates is a
bisector, so every one of these was a plate deciding something different about
two neighbouring samples. Three separate places did it, and all three are the
same mistake: **a branch on which plate is nearest, taken where the features
are at full strength.** Which plate is nearest is not a fact about a place, it
is a fact that flips along a line, and anything read off it flips with it.

**The arc against the trench.** Where ocean meets continent the ocean dives,
so the trench belongs to its side and the volcanic arc to the other. That was
written as `match (near.continental, far.continental)`, with `(true, false)`
lifting the crust by 0.16 and `(false, true)` dropping it by 0.85. Those two
arms meet at the bisector, where `close` is 1 and the force is greatest, so a
full arc stood against a full trench with nothing between: a step of 1.01 in
`land`. It is now one profile read off a signed distance, positive toward the
continental plate, so the trench rises into the arc across `ARC` of angle.

**The regime flipping at a junction.** Where a third plate is as close as the
second, which pair the margin belongs to changes from sample to sample. The
pair sets `normal`, `normal` sets the sign of `motion`, and the sign of
`motion` chooses convergence or divergence, so a trench sat against a ridge
along the line where the second and third plates tie. `boundary` now also
tracks the third and reports how clearly the first two are a pair at all, and
`close` is multiplied by it: at a junction the features fade out instead of
fighting. A triple junction is messy ground on a real planet too.

**The crust changing hands.** The crust level was a blend of the nearest two,
`level(far) + (level(near) - level(far)) * across`. It named a second plate,
and where that plate changed from a continental one to an oceanic one the
blend moved half of 0.98 in one step. There is no fix that keeps a second
plate, so the crust is now an average over every plate within `CRUST` of the
nearest, weighted by how near. No identity appears in it, so nothing can
change under it. It costs a second pass over the sites, which a compare skips
for all but the two or three that are close: `bun run bench` came out slightly
faster than before, at 0.76 ms a patch against a budget of 1.5.

**What is left, and it is not the plates.** The worst remaining step is 21 m,
250 m under water, and it comes from below: `simplex_d` returns an analytic
gradient, `shape` uses that gradient as its domain warp, and the gradient is
discontinuous. The kernel is `0.6 - r²` and only four corners are summed, so
the kernel still has value where the corner set changes and both the value and
its gradient step at every simplex boundary. The warp inherits it, and the
plate machinery multiplies it up. Fixing it means `0.5 - r²` and a rescale,
which changes every world and every noise in the generator, so it is filed
rather than done: the visible artefact is gone, and `worldgen/tests/cliffs.rs`
guards at 60 m, which is far above the 21 and far below the 762.

v3's goldens were updated on purpose. No world exists yet, and the freeze
starts at the first one created (21).
