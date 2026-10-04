# 107. An op of the cells is a gesture, and the server reads no ground (proposed)

Logged 2026-10-04.

The op the core's cells take is a gesture: create, delete or paint, over a
box, in a volume. A platform is the gestures a building plugin composes in
the client, from the ground it reads there: a slab, and the boxes of its
base. The server checks who may change the volume and applies the boxes. It
reads no ground, so the module carries no generator and no field.

**Why.** It follows from 106: a kind of construction is a plugin's, and the
core knows boxes. 97 had the server read the ground to lay a platform, as an
op of a square and a base, and measured it: 8 ms a platform in wazero, and a
world of a field held in the server too, 25 MB that had no home (OPEN). No
rule of the core says where a pillar ends or that a slab meets a hill: a
builder with permission in a volume makes any box in it, and a platform is
some of them. A solid base on a slope is a few boxes a row (85).

**What a volume's bounds are.** The client that first writes in a volume
says how high its plot's ground is, and so where the volume starts and ends.
The server keeps that with the volume and holds it within the build band;
every client reads it from the world, and none works it out again.

**What stays of 97.** The rule is written once, in Rust, and the server runs
it in the module: applying a gesture to a volume is `voxel`'s on both sides.

**Rejected:** a platform as an op of the core, a square and a base, which
puts a kind of construction in the core and the ground in the server.

**Lives in:** ROADMAP.md § Plugins: native; OPEN.md.
