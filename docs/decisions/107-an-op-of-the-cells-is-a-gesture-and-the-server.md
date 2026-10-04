# 107. An op of the cells is a gesture, and the server seats the volume it lands in (proposed)

Logged 2026-10-04.

The op the core's cells take is a gesture: create, delete or paint, over a
box, in a volume. A platform is the gestures a building plugin comes to in
the client, from the ground and the feet it reads there (105): a slab, and
the boxes of its base. The server reads the ground once for a volume, when it
is first written in, to say where it starts and ends. From there it checks
who may change the volume and applies boxes.

**Why a gesture, and not a platform.** It follows from 106: a kind of
construction is a plugin's, and the core knows boxes. No rule of the core
says where a pillar ends or that a slab meets a hill: a builder with
permission in a volume makes any box in it, and a platform is some of them.
What is kept and logged is boxes, so a volume says the same under any
generator: the day the ground's sum changes (OPEN, the simplex kernel), a
platform kept as a square and a base is laid somewhere else, and one kept as
boxes stands where it stood. A solid base on a slope is a few boxes a row
(85), and an op that reads no ground costs microseconds (97).

**Why the server reads the ground.** The ground is the core's, and the core
is the same Rust on both sides (97, 108): the generator is in the module.
Where a volume starts and ends follows from the ground under its plot, and
bounds a client says are bounds a client may lie about. Reading that ground
costs 5 to 10 ms in wazero (97), once for a volume, and what it gives is kept
with the volume.

**What it changes in 97.** A platform is no op of the core. The rest stands:
the rule is written once, in Rust, the server runs it in the module, and
applying a gesture to a volume is `voxel`'s on both sides.

**Left for later.** A world of a field holds its field in the server to seat
a volume, and waits for fields to have a home (OPEN).

**Rejected:** a platform as an op of the core, a square and a base, which
puts a kind of construction in the core and makes what is kept follow the
generator; a volume's bounds said by the first client to write in it; a
server with no generator, which every law of the core that stands on the
ground would bring back.

**Lives in:** OPEN.md; ROADMAP.md § The cells.
