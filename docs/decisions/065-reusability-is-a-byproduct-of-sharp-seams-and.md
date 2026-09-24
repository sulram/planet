# 65. Reusability is a byproduct of sharp seams, and the seams are held by a script (decided)

Logged 2026-09-24.

The question was whether the crates could serve a second engine: a flat
Minecraft, no heightfield, no sphere. Reading the graph answered it: `render`
imports no sphere and still cannot draw without one, because `scene::Frame`
carries a planet radius, a moon and a sun, and the per view uniform every
shader shares is "camera from the planet centre". Decoupled by dependency and
coupled by vocabulary, which is the leak a crate graph cannot show.

**The rule is not "make the engine general."** An abstraction with one
implementation is a guess paid on every change, and an engine built before its
second game is how the first one never ships. Every seam that exists today was
forced by a real second consumer: `Grid` from `QuadSphere` when a small world
was needed (49), the command and event seam when the desktop needed a second UI
(46), `Source` when Earth had to sit beside plates (44). None could have been
drawn earlier, and each was right.

**So the rule is three disciplines and a reading pass**, in CLAUDE.md: a seam
earns its existence with two implementations, one real; dependency direction
is law and `bun run docs` holds the allowed graph, so a new arrow is a decision
before it is an edge; a crate speaks only its own nouns, which is what catches
the `Frame` leak that the graph let by. At the end of a milestone the public
surfaces are read once and the drift goes to OPEN; rewriting waits for the
consumer that pays for it.

**The volume work is the second engine.** Cubic cells in an integer box, a
greedy mesher, cube collision: that is a flat voxel world, and it is pulled by
M1.75. Written with no `topology` dependency, seated on the body by the client,
it is the reuse the question asked for, at no extra cost. The generator, the
sky, the sea and the moon stay the planet's and are kept out of the seams the
rest passes through, rather than being generalised.

Rejected: a calendar rule ("once in a while, make packages more reusable"),
which has no consumer to say what reusable means and fights "milestones pull
features"; a topology trait or a world kind enum now, each with one variant;
a flat mode, which DECISIONS 04 and the M1 list already refuse; leaving the
graph unchecked, which is how a rule becomes a wish.

**Lives in:** CLAUDE.md § How the engine stays reusable; `scripts/docs.ts`.
\n
