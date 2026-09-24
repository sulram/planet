# 35. Grass in reach tiers (decided)

Logged 2026-09-21.

Grass went through a flat candidate grid first: one tuft per block, thinned by
the trailing zeros of its address, with a fixed instance budget that dropped
whole patches. It read as sparse, ended 50 m out and showed a lattice far away.
Now tier `k` holds one jittered tuft per `2^k` half blocks and reaches
`20 m * 2^k`. Spacing and reach double together, so the count per tier is
constant (about 20 000 in the full disc) and screen density stays level out
to 640 m; far tufts grow in the shader to stand for the meadow between them.
A patch `j` levels short of the finest is only drawn from about `27 * 2^j` m
(it splits otherwise), so it carries tiers from `j + 1` up: 1365 tufts, 55 KB.
That bound ties `GRASS_TIER_0_REACH_M` to `SPLIT_DISTANCE`; raise one and the
other must follow. Measured: WASM worst frame of the descent 8.41 to 9.03 ms
(budget 12); native 1280x720, 1.37 to 1.81 ms a frame over shadows alone.
