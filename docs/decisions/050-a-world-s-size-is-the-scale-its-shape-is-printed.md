# 50. A world's size is the scale its shape is printed at (decided)

Logged 2026-09-21.

The generator samples the unit sphere by direction and answers in metres, so
the same seed gave the same heights whatever the body was. At `2^4` that is
relief 125 times the radius: a 5 m planet with its ground 523 m below it.

The fix is not inside the generator. A version is written once, in the metres
of the **reference body** - the largest quad sphere there is - and frozen
there forever, because a world's terrain may never shift under its builds.
The body's size is a scale applied at the `Generator` boundary: metres going
in are divided by it, metres coming out are multiplied. Footprints, cave
depths and ground heights all cross that one seam.

**The factor is `side / 2^16`.** Both sides are powers of two, so it is exact,
and at the reference size it is exactly `1`. Multiplying by one changes no bit,
so the largest world is untouched and the golden hashes hold without being
regenerated, which is the only acceptable outcome for a frozen generator.

The consequence is the good one: a small world is the same world printed
smaller. The seed keeps its coastline, its plates and its mountains, and they
arrive at the body's own scale, relief holding its share of the radius at every
size. A world proved at `2^10` is therefore the world that ships at `2^16`,
which is what makes proving it there worth anything.

Rejected: scaling only the height that comes back, which leaves the cave field
at 20.9 km inside a 5 m body; teaching each generator version its body's
radius, which rewrites frozen arithmetic and breaks every golden hash for a
number those versions never used; a relief knob chosen per size, which is a
constant tuned against a picture, the exact failure 48 was written about.
