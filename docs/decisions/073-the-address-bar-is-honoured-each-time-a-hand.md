# 73. The address bar is honoured each time a hand changes it, and the picture waits behind a veil for the world the page asked for (decided)

Logged 2026-09-28.

The page hands the engine the address bar's place each time `page.url`
changes: when a link brings you in, when a place is pasted over the bar, when
another world is entered, and never when the page itself writes your pose
there as you move. The engine view walks there once it stands in the world
the page asked for. Until then, and while a field is fetched, a blurred veil
with a word on it covers the picture; it lifts on the world you asked for, at
the place you asked for, once the streamer has nothing left to build there.
The client says so with `settled`, the streamer's own word for an update that
found nothing missing, the criterion `settle` already stops on for a headless
shot; lifting on `recipe_changed` alone showed a body hanging over the sea for
the frames the ground took to arrive.

Pasting a pose into the address bar did nothing: the page read the bar once,
at birth. SvelteKit's `replaceState`, which the page uses to write your pose,
leaves `page.url` alone, while a hash typed into the bar comes back as a
`popstate` that updates it, so `page.url.hash` is exactly what a hand put
there, and a new URL object each time makes the same text a fresh request.
Meanwhile the first seconds of every visit showed the engine's own planet,
seed 1, built before the page's recipe arrives, under an OFFLINE badge. It
read as the wrong world taking long to connect. The wait was the field, and
the walk to the address never waited for the link, it waited for the ground.

Decision 62 stands: where you stand after a recipe change is the engine's,
and the page never re-sends what it wrote itself. What changes is who counts
as a hand: a paste and a navigation do, and each is honoured for the world
the page asks for at that moment, so a pose in another world's link waits for
that world instead of landing in the one being left. A recipe the engine
changes at the keys is not one the page awaits: it lifts no veil and drops
none. A recipe the engine refuses shows a message instead of a spinner that
never ends, and ground this instance does not serve says so instead of
showing another planet.

**Rejected:** a `hashchange` listener beside the one-shot arrival, two
mechanisms for one fact; comparing the pasted text with where you stand,
which turns a second paste of the same place into a no-op; honouring the bar
before the world stands, which walks you to a place in the wrong world and
lets the respawn decide whether you keep it; not building the engine's first
world at all, which is the right end and a change in `shell-web` and `client`
(ROADMAP).

**Lives in:** ARCHITECTURE.md, WORLD.md,
`apps/web/src/lib/engine/Stage.svelte`,
`apps/web/src/lib/engine/EngineView.svelte`.
