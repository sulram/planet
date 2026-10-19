---
group: problems
since: 2026-10-04
context:
  - /server/internal/world/recipe.go
  - /proto/planet/v1/world.proto
---

# A hosted world is always `2^16`

A recipe names its own `sector_bits` (49), and neither the world folder, the server's recipe nor the wire's carries it: a smaller world is founded only in a client alone. Chat's world half measures by whatever the recipe says

**Context:** `server/internal/world/recipe.go` · `proto/planet/v1/world.proto`.
