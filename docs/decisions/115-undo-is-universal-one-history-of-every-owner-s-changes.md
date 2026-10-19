# 115. Undo is universal: one history of every owner's changes, each owner taking its own back (decided)

Logged 2026-10-19.

A hand has one history: every change that landed, of every owner, the cells'
and the parts' and every owner's to come, in the order it landed, taken back
and put back as one list. Ctrl Z and Ctrl Shift Z are the core's keys, and no
plugin asks for them. The host on the client keeps the order and which owner
each change was of; each owner keeps the changes themselves and takes its
own back, here and in the world, through ops of its own, `take_back` and
`put_back` under its name.

**Why one history.** Marlus: undo is universal, not building's alone, for
every item of today and of what comes. A person undoes the last thing they
did, whatever it was; two undo stacks are a thing to learn, and a plugin that
keeps its own is a plugin that forgets to.

**Why the order is the client's and the changes the owners'.** The module's
bridge lets an owner keep and tell under its own name alone (102, 110): a
history that reached into other owners to take their changes back would
speak for them, which the bridge refuses by design. So the history keeps no
change. It keeps which owner each change was of, on the client, where the hand
is; the owner keeps the change and its inverse, as the cells did already and
the parts do now, and the world keeps each owner's stack in the same order,
since a session's ops land in the order asked. A change the world refused
leaves the list by the number it was asked under, so the two stay in step.

**Sets of one part are one change.** A drag sets a part every frame and the
world hears it ten times a second; the history and the owner alike count sets
of one part in a row as one change, back to how it stood before the first.

**What a new owner does.** Its ops that change state tell the host they
landed through the host's words, which enter them in the history; it offers
`take_back` and `put_back`; and it tells whoever is near what the world is
then, in its own events, as every change is told.

**Rejected:** an undo per plugin, which the brief's building had and this
retires; a history owner in the module that reaches into the others, refused
by the bridge; undo keys asked for by a plugin, which took them from every
other.

**Lives in:** PLUGINS.md § The path of a change; GLOSSARY.md;
`crates/client/src/history.rs`; `crates/world/src/cells.rs`,
`crates/world/src/parts.rs`.
