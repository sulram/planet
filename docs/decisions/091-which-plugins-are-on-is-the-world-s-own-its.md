# 91. Which plugins are on is the world's own: its admin chooses, and the world says what it speaks (decided)

Logged 2026-10-04.

A version carries plugins; a world says which of them are on. The config that
lists a version's plugins says whether each starts on, and everything in the
first package does. Whatever is compiled in can be switched: the world's admin
chooses when founding it and may choose again, and the choice lives in the
world folder, so an upgrade carries it. A world answers anyone who asks with
what it is: the version of the engine, the version of the wire, and each
plugin that is on with its own version.

**Why in the world.** Two worlds on one version must be able to differ, or
comparing two ways of building means publishing an image for each combination.
mundos holds no world logic, so it is not a variable there. And founding is
already the moment an admin decides what a world is (87): its shape, and now
its layers.

**Why the world says what it speaks.** The browser needs it to mount the
panels of the plugins that are on. An installed client, a desktop or a headset
paired by a code, visits worlds of other versions and needs it before it
enters: what it lacks it can refuse, or leave undrawn. The web pays nothing
for this, since each world serves the client built with it.

**What is promised, and what is not.** planet is at version zero and a
breaking change is allowed: nothing here promises that an old world opens on a
new version, or an old client in a new world. What is in from the first day is
what is cheap now and dear later: the versions are said, the wire's envelope
carries a plugin's name beside its payload so the core's part changes rarely,
and a plugin moves its own store forward when it starts. mundos's upgrade is
the net under it: the move happens on a copy, and the old generation is the
way back.

**A plugin switched off** keeps its store untouched and sends nothing.

**Rejected:** the config at build alone, one set of plugins a version; a
variable for each instance in mundos; a toggle for each person, which is a
viewer's choice of what to draw and belongs to a front end; a promise of
compatibility before there is a second version to keep it with.

**Lives in:** ARCHITECTURE.md § The core and its plugins; BRIEF.md.
