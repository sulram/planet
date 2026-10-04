# 97. A plugin's rules are Rust, and the server runs them as WASM (decided; a platform is a plugin's gestures, 106)

Logged 2026-10-04.

What an op does to what a plugin owns is written once, in Rust. The client
compiles it in, and the Go server runs the same code as a WASM module through
wazero, the generator with it. Go carries, checks who, keeps and relays, and
decides nothing about a cell. This confirms 09 for the generator and is how
93's rule is written once.

**Why.** A Go package beside the Rust writes every rule twice: a gesture
today, a ramp, a brush and a second way of building after it, and two
implementations diverge (09). A platform's op is a square and a base (78), so
its cells follow from the ground, the server reads the ground, and the
generator is Rust.

**What it costs, measured.** `voxel` and `worldgen` as one module of 132 kB,
in wazero 1.12 with its compiler, on an Apple M5 Pro (arm64), beside the same
code built native:

| | Native | In wazero |
|---|---|---|
| A stroke of 8 by 8 cells, created and deleted | 0.6 µs | 3 µs |
| A stroke of 64 by 64 cells, created and deleted | 35 µs | 118 µs |
| The ground under a plot, 65 by 65 samples at full detail | 3.9 ms | 7.8 ms |
| A platform of 64 a side on pillars, ground read and cells laid | 3.5 ms | 7.8 ms |
| The same, solid | 3.6 ms | 8.4 ms |

Over 210 places of a generated world, the ground under a plot takes 4.9 to
9.9 ms, 6.0 in the mean. The module compiles once in about 30 ms, starts in
under a millisecond and holds 1.25 MB. An op that reads no ground costs
microseconds; one that reads it costs an eighth of the actor's tick of 66 ms.

**What it asks.** wazero's compiler runs on amd64 and arm64, the two an image
is built for, and needs no CGO. Its interpreter is a hundred times slower,
0.7 s a platform, and is no place to run a world. A world of a field holds
the field in the server too, which is unmeasured until fields have a home
(OPEN). The numbers come from a spike outside the repository; the module
carries a bench of its own when it is built (ROADMAP).

**Rejected:** a Go package for each plugin's rules; CGO to link the Rust; the
client trusted to send the cells an op wrote; ops kept and never applied by
the server, a log the client replays.

**Lives in:** PLUGINS.md § The path of a change; ARCHITECTURE.md § The server;
ROADMAP.md § Building; GLOSSARY.md.
