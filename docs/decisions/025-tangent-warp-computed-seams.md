# 25. Tangent warp, computed seams (decided)

Logged 2026-09-20.

Settles the OPEN question on the pre-distortion mapping: `tan(s * pi / 4)`.
It is one line, exactly invertible with `atan`, deterministic through `libm`,
and bounds the block edge between 0.35 m and 0.5 m. Seams are derived from
integer cube geometry rather than a hand written table, so the mandatory
property tests check geometry, not typing. Rejected for now: Everitt and other
equal area warps (more math for a gain that does not matter at this size).
