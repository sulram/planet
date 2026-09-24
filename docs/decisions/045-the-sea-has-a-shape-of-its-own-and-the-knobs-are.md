# 45. The sea has a shape of its own, and the knobs are at the address (decided)

Logged 2026-09-21.

A field world drowned nothing. The Channel is 40 m of a body whose range is
11 km, so a depth taken in proportion put it under 0.7 m of water, and Britain
grew a land bridge to France; the North Sea, the Baltic, the Persian Gulf and
the Yellow Sea went the same way. The land had been given an exaggeration and
the sea had not. `sea_curve` is the exponent the depth follows down from the
shore: `1` is the old proportion, `0.45` is the default and puts the Channel
42 m down while the abyssal plain still arrives at all of `ocean_depth_m`.
`sea_level_m` moves the coastline on the source body, which is what
`sea_share` does for a generated world and could not do for a field: raise it
to drown the lowlands, lower it to walk out onto the shelves, as the Channel
was walked in the ice age.

`tests/earth.rs` now asserts both ends, because they pull against each other:
five shallow seas have to be deep enough to be seas, and the South Pacific has
to still reach the floor the recipe names. The first fails at 0.7 m on the old
curve, which is what makes it a test and not a restatement.

The knobs then had to be reachable. Every param of a recipe that changes what
a planet looks like is a slider in `/play` and a parameter of its address, so a
planet stays shareable and "create world" saves the one that was previewed.
`KNOBS` in `$lib/world.ts` is the single table: range, step, default, and which
shapes a knob means anything for, because the URL, the sliders, the form and
the clamp on the server all have to agree and agreeing is cheaper than
checking. The server clamps again on create: the form is a suggestion, the
range is the rule. A knob left at its default is absent from both the address
and the recipe, so a recipe carries only what someone chose. `Slider` gained
`onchange`, which fires when a drag ends, so the picture answers per frame
while the address is rewritten once.

**The planet stays at `2^16`.** Growing it was tried, measured and reverted.
Each bit doubles the radius, and at `2^19` (167 km, 1049 km around) Chile is
4.7 km wide instead of 590 m and Thailand is a place rather than a ridge. But
the same change takes the vertical exaggeration from 48 to 6 and the planet
reads flat, and the atmosphere, tuned as a uniform shell for a 20.9 km body,
turned every distance white until its density and height were re-paired. The
size and the drama are one dial, not two, and `relief_m` can buy the drama
back (2500 at `2^18` gives 22 times and four times the ground). The finding is
in OPEN.md with its numbers, against the question that was already there.

Rejected: lifting shallow water by clamping a minimum depth (a step at the
coast instead of a slope); baking a second field of "sea floor" at a different
scale (a second read path, and the curve is one `pow`); knobs as a settings
panel instead of the address (a planet is a recipe, and a recipe you cannot
paste to someone is not one).
