# 87. planet is a world mundos hosts: one world per instance, and who is who arrives signed (decided)

Logged 2026-10-04.

planet is one world per instance, hosted by mundos. mundos, the studio's host
of worlds (`~/Dev/mundos`), creates, addresses, versions and upgrades
instances and says who each person is. A world is born unfounded and its
first admin founds it. Anyone enters, as a guest too; admins and builders
build.

**Removes:** 11, 13, 22, 24, 46, 70.

**Why.** planet was growing a second platform beside the one Marlus already
runs: its own sign in, its own roles, an operator and a backoffice, a list of
worlds, a deploy to a box. mundos does each of those for the Hyperfy fork
today, with one sign in for every world, and two answers to who someone is are
one too many. What is left for planet is what only planet does: the sphere,
and what people do on it.

**What mundos gives a world.** A container from a published version of the
image, a folder that outlives it, a bucket folder for heavy files, an address,
and three variables: its public key, its own origin and the world's name. An
update is never in place: mundos copies the world into its next generation on
the new version, someone walks the copy, and a promotion gives it the address.

**Who is who.** Every load passes through mundos's door, `/enter?host=`, and
comes back with a token in the URL's fragment, or with `guest`. The token is a
JWT signed with Ed25519: the account's id, its name, the world it is for and a
level, good for a minute. The page hands it to the world once, on entering:
the server checks it against the public key and answers with a key, which the
page keeps in memory and shows on the socket and on a founding. So the level
holds for the life of the page, a link that drops comes back with no reload,
and a reload passes through mundos again. The levels are mundos's: `admin`,
`builder`, `signed_in`, `anonymous`. Accounts live in mundos, and the world
holds its keys in memory alone; a role changed in mundos holds on the next
load. This replaces the ticket of 66: what crosses comes
from outside, signed.

**Founding.** mundos creates a world by name and version alone: a planet is
chosen by looking at it, and world logic stays in the world. So a new world
waits for its seed and its source. The first admin to enter sees the offline
preview with its knobs, Earth or generated, and founds the world from what is
on the screen; the recipe is frozen from then on. Until then everyone else is
told the world is on its way.

**Who builds.** An admin and a builder, anywhere. Building is in the client
alone today (76), so the server says the level in `Welcome` and the client
offers the tools by it; when a stroke is an op, the actor checks the same
level. Land, with volumes that say where each person may build (14), is a
plugin wish.

**Rejected:** accounts in planet beside mundos's, two answers to who someone
is and an e-mail in every world; many worlds in one instance, since mundos
addresses, protects, lists and upgrades one world at a time and a second one
inside would be invisible to all of it; a session of the world's own in a
cookie, which outlives the page and keeps an old level until the person passes
through mundos again (mundos refused the same for Hyperfy); the token itself
on the socket, which lives a minute while a field takes longer than that to
arrive on a slow line, and leaves a dropped link nothing to come back with but
a reload, which costs a builder what they built; founding from mundos's screen,
which would put a generator's knobs in a host that knows no world; planet's
own deploy to a box, a second front door on a server mundos owns.

**Lives in:** VISION.md; ARCHITECTURE.md § A world, hosted and § Identity and
levels; DEPLOY.md; BRIEF.md; in mundos, `docs/BRIEFING.md` § Identity inside
the world, and `packages/mundos` in the fork for the same flow in Hyperfy.
