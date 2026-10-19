# 112. The world has one clock, the server's, said in Welcome (decided)

Logged 2026-10-19.

The sun, the moon and the weather are functions of the world's clock, and
online that clock is the server's: `Welcome` carries the server's wall clock
in milliseconds since the Unix epoch, and the client sets its own by it, so
every eye in a world sees the same sky at the same moment. Offline the clock
counts from the world's making, and the sun's phase is the world's: at zero,
mid morning at the recipe's spawn. The client's clock takes every second a
frame measured, a stalled tab's whole gap included; only the body's step is
clamped.

**Why the server's clock, and not a tick.** The sky is a pure function of
time, so one number at the door is enough: no stream, no state in the actor,
nothing a plugin has to know. A reconnect is a new Welcome and a fresh set.
Drift between the two clocks is the drift of a wall clock, nothing a session
lives long enough to see; a day is twenty minutes.

**Why the phase is the world's.** Two clients set by one server agree on the
sun only if their offsets agree, so the offset is a fact of the recipe (the
spawn), never of when a client was made or turned a knob. A new recipe is a
new world and starts its clock at zero, so it still opens mid morning.

**Rejected:** a clock in every heartbeat, state and a stream for what one
number does; the day's length on the wire, a knob nobody asked for (it is
`DAY_S` in `client`, a recipe param when a world asks); the shells' wall
clocks offline, which would make a headless shot depend on the day it ran.

**Lives in:** `proto/planet/v1/world.proto` (`Welcome.clock_ms`);
`server/internal/world/actor.go`; `crates/client/src/lib.rs` (`clock_s`,
`face_the_sun`).
