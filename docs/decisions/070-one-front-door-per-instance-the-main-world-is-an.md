# 70. One front door per instance: the main world is an instance record (decided)

Logged 2026-09-25.

An instance opens on one world. `/` is that world, full screen, the way
Cryptovoxels is one world and a Hyperfy instance is one world; the list of
worlds and the planet explorer are operators' tools and move under
`/backoffice`, where the operator barrier already stands. Which world is the
front door is a field of a new `instance` collection that holds exactly one
record, seeded on start and refused a second by a hook, so the web, the
desktop (46) and any agent open the same door by reading one row. An operator
chooses it on `/backoffice/worlds`, and only operators create worlds. Signing
in happens inside the world, as a dialog over the existing magic link
actions, so nobody leaves the planet to get a name.

**Why one door.** The vision is a planet people live on, not a catalogue of
planets. A public list invited every visitor to make one more empty world and
left the main one no more prominent than the last experiment. The SaaS of 11
is unchanged: a tenant is an instance, an instance has a front door, and more
worlds inside a tenant arrive through portals, never through a list.

**Why a record and not a flag on `worlds` or an env variable.** A boolean on
`worlds` needs a hook to keep it single and says nothing else about the
instance; the same row will carry what is the instance's and not a world's,
its name, its welcome, its default avatar set. An env variable is set by whoever
deploys, and the front door is chosen by whoever operates.

**Why a world stays reachable by link.** `/w/<id>` is unlisted, not locked: a
shared link that stops working is a worse surprise than an unlisted world,
and portals will want the address. A lock is one line in that load if an
instance ever needs it.

**Why the login is a dialog and not a page.** The person is already standing
in the world; the email and the code fit in a dialog, and the actions of
`/login` and `/login/code` answer it unchanged through the form enhancement,
which never leaves the page. The link in the email still lands on
`/login/verify`, which sends the person back to `/`.

Rejected: a public list of worlds; a `main` boolean on `worlds`; the main
world in the environment; locking `/w/<id>` to operators; a login page the
world sends people to.

**Lives in:** ARCHITECTURE § Identity and permissions and § Clients and UI;
`server/migrations` (`instance`); `internal/cold` (the seed and the hook);
`apps/web` routes `/`, `/w/[id]`, `/backoffice/*`; GLOSSARY (Instance, Main
world).
