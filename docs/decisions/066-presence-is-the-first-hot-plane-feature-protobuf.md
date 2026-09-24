# 66. Presence is the first hot plane feature: protobuf on one socket, a world actor relaying stances, a ticket for identity (decided)

Logged 2026-09-24.

Marlus asked for the multiplayer system now, ahead of the rest of M1, so
the M2 item "two people see each other" is pulled forward the way 22 pulled
the first vertical: the M1 discipline list in CLAUDE.md is relaxed for
presence alone, and everything else in it stands.

**The wire is protobuf.** `proto/planet/v1/world.proto` is the single source;
`bun run proto` generates Rust with prost and Go with protoc-gen-go, both
committed, so building needs no generator and CI proves the tree is the
schema's. Chosen over flatbuffers because nothing on this wire is large enough
for zero copy to matter, prost and the Go module are the mature pair, and
protobuf-es covers TypeScript when `packages/` needs bindings. Chosen over a
hand written codec because the schema grows with ops (M3) and entities (M5),
and a codec kept by hand in two languages drifts. Chosen over generating in a
`build.rs` because the Go side needs a generator run anyway, and one command
that writes both trees is one thing to know.

**A stance is an address.** What presence carries is the controller's own
state, sector and fractional blocks and a height, never a world position: the
server stays authoritative by address and the same message will carry an
agent. A `float` holds a `2^16` sector to 2 mm. The moon has a column too, the
one its radial passes through, as the place code already does it.

**The world actor relays, and owns nothing else yet.** One goroutine per
active world, started by the hub on the first session and retired after the
last; every session speaks to it through an inbox and hears from it through a
bounded outbox, so the actor never locks and never waits for a client. What
moved since the last tick goes out at 15 Hz in one frame encoded once, own
stance included and ignored by its sender: cheaper than a frame per recipient.
A quiet room still hears a heartbeat every two seconds, and a client sends one
too, so a dead link is noticed within seconds on both sides. The actor holds
no `world.db` yet: chunks pull it in.

**Identity crosses as a ticket.** The web keeps the session token in an
httpOnly cookie the engine cannot read, and the server's origin is not the
page's, so the cookie would not ride the socket anyway. The page's server side
mints a ticket with the person's own token, hands it to the engine, and the
socket redeems it on the way in, before the hub: one use, one minute, worth
one connection. No ticket is a visitor. The hot plane never sees PocketBase:
`Identity` is two strings.

**The shell owns the socket, the client owns the protocol.** `shell-web`
opens a WebSocket and pumps frames; `client` says hello, keeps the peers,
decides when a stance is worth sending, and reports over the seam. An agent
drives the same three calls with no renderer. The page decides when to connect
and when to try again, because it is the one holding a fresh ticket.

**A peer is one figure among figures.** The local body and every peer are the
same `Figure` over one shared set of clips, posed from a `Motion`; a peer's
motion interpolates in world space between its last two stances, drawn a tick
and a half behind, so a walk across a sector seam never interpolates through
the seam. Avatars load once per asset reference and are worn by any number of
bodies, which is also how a person's own avatar will load: the reference is
the same kind of string.

Rejected: the session token on the socket (readable by scripts, and not sent
cross origin); an origin check on the socket (nothing rides it a check would
protect); a frame per recipient (N encodes for one tick); interpolating in
address space (wrong across a seam); the desktop shell in this change (it
needs the seam's cold plane work first, DECISIONS 46); nametags in the world
(a render feature, listed in ROADMAP).

**Lives in:** ARCHITECTURE § Two planes and § Clients and UI; `proto/`,
`crates/protocol`, `server/internal/world`, `server/internal/cold/hot.go`,
`client::{session, peers, wardrobe, figure}`, `shell-web`, `EngineView.svelte`.
