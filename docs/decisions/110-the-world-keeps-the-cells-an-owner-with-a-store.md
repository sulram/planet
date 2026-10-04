# 110. The world keeps the cells: an owner with a store, asked through a fourth name (decided)

Logged 2026-10-04.

The cells in the world are an owner as a plugin's world half is: Rust in the
module, reached by the envelope under the name `cells`, their ops said as
data with a level each, always on and switched by nobody. An op asked with an
id is answered, landed or the code of why not. What an owner keeps is rows of
a key and a value in a SQLite file of its own in the world folder: it reads
them through a fourth name of the bridge, `host.ask`, and what it writes goes
back as replies the server keeps when the call returns, all of it or none.

**Why an owner, and the envelope.** The server already routes an envelope to
its owner and asks the level before the owner sees the op (93, 98). The cells
needed nothing more of it, so Go learned no cell: it carries bytes, keeps
rows and answers ids. A second system of the core, bodies, enters the same
way.

**Why a store asked from inside the call.** A change reads the chunks it
reaches and writes those it changed. Which chunks those are is the cells' own
sum, so the server cannot send them with the call, and a module that held
every volume in memory would need a ceiling no world fits under. Asked from
inside, the store is a service with no feature (99): the next owner that
keeps something uses it as it is.

**What the module holds between two calls.** Where every volume stands, read
from the store once, and what each session may take back. A new instance
reads the first again and does without the second. It is the first state the
module holds; a turn of its own, when bodies are simulated there, is one more
call.

**Who is told, and who arrives.** A change goes to the sessions whose bodies
are within reach of the volume (107). A client says what it holds, each
volume at a version, and is shown whole what stands near it that it lacks or
holds stale: the same question serves who arrives, who comes back, and who
missed an event.

**A change made at once.** The client makes a change as it asks for it, and
holds it pending until the world answers. What others changed meanwhile goes
under it, in the world's order, and a refusal takes it away.

**Taking back, with others around.** The world keeps, for each session, the
cells before each of its changes. Taking one back restores a cell that is
still as the change made it, and leaves a cell someone else changed since.
What is kept for it goes with the session and with the module's instance:
the log that outlives both is ROADMAP's.

**Left out.** A world shaped by a field takes no volume: its server holds no
field to seat one by (OPEN), and the client says so before it asks.

**Rejected:** the cells as messages of the wire the server reads, a cell
known to Go; the chunks an op reaches sent with the call, the cells' sums
mirrored in Go; every volume held in the module, a ceiling of memory as the
size of a world; a change told to everyone; a volume's history taken back
whole over what others built since.

**Lives in:** `proto/planet/cells/v1`; `crates/seat`; `crates/world/src/cells.rs`;
`crates/client/src/cells.rs`; `server/internal/store`; `server/internal/module`.
