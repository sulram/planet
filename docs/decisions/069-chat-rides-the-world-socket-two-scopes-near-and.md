# 69. Chat rides the world socket: two scopes, near and world, a line is never stored, and @here is a place to teleport to (decided)

Logged 2026-09-24.

Marlus asked for text chat, so the M1 discipline list is relaxed once more,
the way 66 relaxed it for presence, for chat alone. Chat is two messages on
the protocol of 66, `Say` up and `Said` down, relayed by the world actor the
way it relays stances. A line has one of two scopes: `near`, everyone within a
radius on the same body, or `world`, every session in the world, the planet,
the moon and any body that comes. There is no chat between worlds. A line is
never stored. A line said with `here` carries the speaker's place, filled in by
the actor from the stance it holds, and clicking that place is a teleport.

**Why the socket and not a chat server.** The actor already keeps every
session's last stance, so `near` is a filter it applies for nothing; a chat
server beside it would need every stance mirrored into it to do the same, which
is presence done twice. An agent is a client without a renderer, and a line on
the protocol is the first thing an agent can say to a person, where a line on
IRC is invisible to it. The desktop hears chat the day it opens the socket.
One wire, one identity, one process.

**Why two scopes and no more.** A world is the room: its bodies are one
place, reachable by flight, and the people in it are the people you can meet.
An instance-wide channel belongs to no actor, so it would be the hub's first
feature and the first thing to say "everyone" when the vision says "near".
Federation across instances is further still.

**Near is measured by the actor.** Same body, and within a radius of the
speaker: the great-circle distance on the body's datum and the difference in
height, taken together as one distance, in blocks. The actor turns an address into a direction with the one
cube-sphere mapping `topology` holds, mirrored in Go as one function and
pinned by a test to values the Rust side prints, so the two cannot drift
unnoticed. The radius is one constant in the actor to start, 64 blocks, and
becomes a world setting when a world wants another.

**The place rides as a field, filled by the server.** `Say` carries a flag,
never a code: the actor copies the speaker's last stance into `Said`, so a
client cannot claim to stand where it does not, and the receiving client
turns the stance into a place code the way it does its own. The UI turns the
typed `@here`, `@aqui` in Portuguese, into the flag and draws the place as a
chip beside the name; an agent reads the field and parses nothing. Clicking
the chip issues `GoTo`, which makes `GoTo` travel as well as arrival: the
seam's comment saying otherwise is rewritten in the change that lands chat.
Teleports and portals are wanted (ROADMAP M6+), and this is the first one.

**A line is not a change to the world**, so the database never sees it. No
history rides in `Welcome`: a late joiner hears from now on. A ring of the
last lines, if a UI ever wants one, lives in the actor's memory and dies with
the actor.

**Limits live in the actor from the first day**, because spam is the first
thing to arrive: a line is at most 500 characters, counted as a person
counts them and never as bytes, and a session says at most five lines in five
seconds; past either the actor drops the line. The UI holds the same limits,
with a count that appears as a line grows long, so a person never meets them. A line carries the
session, and the client knows the name from its peers; a visitor speaks as a
nametag will show it. Chat lands after nametags (ROADMAP M2), because a line
from nobody is noise. A mute is a role, and roles are per world (M4): until
then moderation is OPEN.

Rejected: IRC, Matrix or XMPP behind a bridge (a second process, identity and
socket, and a bridge that mirrors presence to get `near`); a hosted chat
service (not free software end to end); PocketBase realtime on a `messages`
collection (the hot plane through PocketBase, and it never sees a stance); an
instance-wide scope; `near` filtered by the listener (a scope the speaker did
not choose, and a mute has to be server side anyway); the command parsed by
the server (a command is a UI word, and UI words are localized); the place
written into the text as a code (a field is drawn by a UI and read by an agent
without parsing); history in `Welcome`.

**Lives in:** ARCHITECTURE § Two planes; `proto/planet/v1/world.proto`;
`server/internal/world`; `client::chat` and the seam; the web chat panel;
`ui-native`.
