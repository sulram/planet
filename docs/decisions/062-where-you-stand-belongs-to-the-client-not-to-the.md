# 62. Where you stand belongs to the client, not to the page (decided)

Logged 2026-09-22.

Making a world over the Earth field dropped people into the ocean. Two things
were doing it, and only one of them was the one being looked for.

**The page was the near cause.** `EngineView` re-sent the pose after every
`recipe_changed`, so the client's respawn on dry ground was undone a moment
later by a `go_to` carrying the address the person held in the *previous*
world. The comment there said why, and it was right about the goal: turning a
knob should leave you standing where you were. It was wrong about who can
decide that. An address names a column, and what sits under that column
belongs to the recipe, so the same seven characters are a hilltop in one world
and open water in the next. Over a field of the Earth the odds are 71% water.

Rejected: comparing the two recipes in the page and re-sending the place only
when the fields that shape the ground agree. That list is `seed`,
`sector_bits`, `generator_version`, `source`, `sea_share`, `continent_scale`
and `sea_level_m` today, it has to be edited every time a knob is added, and it
lives in a language that cannot check itself against the generator. It also
cannot answer the case with no previous world at all: a link shared before
someone raised `sea_level_m` arrives underwater, and no comparison of recipes
in the page will ever know that.

So the client decides, where the generator already is. `regenerate` keeps the
place while `standable` says the ground under it is clear of the waves, and
spawns otherwise. A flyer keeps its altitude, lifted if the new ground rose
through it. The page sends the address bar's place once, for the world that
link was written for. One check replaces an enumeration, and it ages by itself
as knobs are added.

**`spawn_point` was the older bug, and the worse one.** It searched a 25 by 25
lattice of sector 0 alone and, finding nothing dry, stood in the middle of that
sector. A face of a quad sphere can be entirely ocean: seed 10 opens on 476 m
of seabed, and over the Earth field one face is most of the Pacific. It now
walks every sector, sector 0 first so a world that had an answer there opens
where it always did, and its last resort is the highest ground it saw anywhere
rather than the centre of a face, because the shallowest water is the likeliest
place to find a shoal. The worst case is six lattices instead of one, paid once
when a world is made and never inside a frame; `bun run bench` is unmoved,
since no budget it measures covers world creation.

Both are pinned by tests in `client/tests/place.rs`: a place survives a knob,
never survives into water, and every seed from 1 to 23 opens on dry land.
