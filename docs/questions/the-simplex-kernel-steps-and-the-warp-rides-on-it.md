---
group: problems
since: 2026-09-23
---

# The simplex kernel steps, and the warp rides on it

`simplex_d` uses `0.6 - r²` over four corners, so value and gradient both jump a little at every simplex boundary, and `plates::shape` warps its domain by that gradient. Worst seen: 21 m of seabed, 250 m under water. `0.5 - r²` is continuous and costs amplitude; either way every world and every golden changes, so it waits for a reason to spend that

**Context:** DECISIONS 63 · `worldgen/tests/cliffs.rs` guards at 60 m.
