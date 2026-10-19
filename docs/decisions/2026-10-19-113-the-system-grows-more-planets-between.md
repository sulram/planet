---
date: 2026-10-19
lives_in:
  - ROADMAP.md
  - ROADMAP.md#the-shape
---

# The system grows: more planets between the planet and the moon

**Status:** decided; drawn ahead, built when pulled.

**Decision:** A world is a system of bodies, not a planet and its moon: more planets orbit
between the planet and the moon, each a body of its own that is walked, flown
to, built on and lit by the same sun, on the same clock (112). Players ask
for it, and Marlus wants it. Each new body enters as the moon did (31): a
sphere with a real position on rails and a real radius, its own gravity
field, its own terrain streamer over the shared quadtree, and a seat of its
own for what is built on it.

**Why now, as a decision.** The seams that hold a second body are cut by the
moon already: `Body` in the terrain, the controller's field, a seat per body.
A third body is a test of those seams, and what it finds crooked is
straightened before the system holds many. Naming it now keeps the next cut
of `scene::Frame` (ROADMAP: the sphere leaves the seam) from hard coding a
planet and one moon.

**What it waits on.** The shape of a list of bodies in the recipe, how an
orbit is told, and what each body's generator is: a sketch first, then the
first body beyond the moon pulls the code. Nothing of it is built today.

**Rejected:** a second star, or a planet outside the moon's orbit, until the
inner ones stand; bodies that are only sky, drawn and never reached.
