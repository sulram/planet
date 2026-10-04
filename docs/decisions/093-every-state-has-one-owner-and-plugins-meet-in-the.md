# 93. Every state has one owner, and plugins meet in the core's words (decided)

Logged 2026-10-04.

Every piece of a world's state has one owner, the core or one plugin, and the
owner alone changes it. What crosses a seam, between a front end and the
client, a client and the server, the core and a plugin, is one of three
conversations: an op asks an owner to change what it holds; a question asks
for an answer and changes nothing, a hook when the core asks its plugins and a
reading when anyone asks an owner what it holds; an event says what happened
to whoever listens. A plugin imports the core and never another plugin: two
plugins meet in the core's words, who (the account and its level), where (the
address) and what (the name of the action).

**Why owners and messages.** In the fork a system reaches any other by its
name on the world and calls what it finds there, which one language in one
process makes cheap. Here a thing done crosses Rust, Go and a front end, and
the server is the authority: what crosses all three is a message with a name.
One owner gives "who changed this" one answer, and makes a plugin switched off
a store left whole (91).

**Why three, and each one thing.** An op changes, and answers only that it
landed or the code of why it was refused. A question answers and changes
nothing, so it is asked as often as anyone likes. An event answers nobody, so
whoever does not care lets it pass. Which of the three a message is says what
may be done with it: an op is checked, logged and taken back; a question is
repeated or kept; an event is dropped. A message that is two of them is two
messages.

**The path of a change is one.** An op arrives. The host asks the permission
hook with who, what and where, before the owner sees the op. The owner applies
its rule, keeps the result and logs the op. An event tells every session. A
plugin has no way around that path, so a rule held on it holds for a hand, a
macro and an agent alike, and "every edit is a permission-checked op" is the
shape of the host and nothing a plugin remembers to do.

**Why the core's words.** Building asks whether an account may build in a box
of the address. Land answers by the plots it holds permissions for. Neither
names the other: the question is the core's, a plot is the address less six
bits (77), and the same question serves the next plugin that puts a thing
somewhere and the next that says who may. A hook has a default answer, the
level for permission, and a plugin over it answers in its place, with the
default to fall back on.

**A rule is written once.** What an op does to what a plugin owns is one piece
of code: the client predicts with it and the server decides with it. How the
server runs it is in OPEN.

**What holds it.** `ALLOWED` in `scripts/docs.ts`: the row of a plugin's crate
names crates of the core and never a plugin's.

**Rejected:** systems that call each other by name, as the fork has them,
across three languages; an ECS as what every plugin stands on, where a world
is a handful of large owners and 03 keeps a light one for entities; a plugin
importing another, which is the answer to "may a plugin stand on another";
events on free topics, which a listener reads only by knowing the speaker; a
permission check written inside each plugin, which the next plugin forgets.

**Lives in:** PLUGINS.md; ARCHITECTURE.md § The core and its plugins;
CLAUDE.md § Invariants; GLOSSARY.md; `scripts/docs.ts`.
