# 81. The eye eases over a step the body takes at once (decided)

Logged 2026-09-30.

On foot the body takes a step whole, in the frame it happens, and the eye the
camera hangs from comes after it on a critically damped spring of 16 per
second: a third of a second and it is there. Only a change of a step or less
is eased, and only along the up of the frame. Everything else the camera
follows as it did, with no play.

**Why.** Marlus climbed a stair of cubes and the view jumped at every cube.
The camera was a function of the body and nothing else, and the body walks in
address space, where a rise of one block is taken in stride: half a metre in
one frame. For the body that is right. Collision, the server and every peer
read an address, and an address has no in between. The jolt is a matter of
the eye, so the ease is the eye's, and the body stays exact.

**Why a spring.** An ease straight at the body starts at its fastest. On a
stair climbed at a walk, a step every tenth of a second, such an eye swings
between 2.7 and 7.7 m/s and the view shivers ten times a second. The spring
starts from rest: on the same stair it keeps between 3.8 and 5.4 m/s, and at a
run within 0.6 m/s of the body. It is solved in closed form, so a long frame
settles further and never past.

**What it costs.** While a stair is climbed the eye rides under where it
stood, 0.6 m at a walk and 1.5 m at a run, and over it on the way down. A
walk up a natural slope of 30% leaves it 0.17 m under. The cost in a frame is
one exponential: `bun run bench` reads as before, 9.25 ms for the worst frame
of a descent against a budget of 12.

**Rejected:** a body that is interpolated, where what is drawn and what
collides part; play in the whole camera, since sideways it loosens the
steering and a flyer moves kilometres a second; easing every change of
height, which leaves a falling body out of the view; easing the lift onto a
platform laid under the body, where the eye would start under the slab; a
ceiling on how far the eye may trail, which brings the jolt back on a stair
taken at a run.

**Lives in:** ARCHITECTURE.md § Clients and UI,
`crates/client/src/controller.rs` (`stand_on`, `settle_eye`).
