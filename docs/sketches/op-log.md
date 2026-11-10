---
title: The op log
group: Work
summary: "A sketch: how the log of ops would be kept so undo, audit and rollback stay cheap"
---

# The op log

A drawing, nothing decided. The wish is in [ROADMAP.md](../ROADMAP.md#the-cells-58-75-to-86-106): an op log, undo, and a volume rolled back to an earlier moment (95). The log would grow without a ceiling and outweigh the chunks long before they matter, so how it is kept would be part of building it.

- **An op is a gesture, not a cell.** One stroke would be one permission-checked op carrying its shape and its parameters, never the thousands of cells it wrote.
- **The log is not the world.** Stored chunks are, by copy on first write, so the log would never be replayed to rebuild anything: it would exist for undo, audit and rollback, which is what would make it safe to compact.
- **Two tiers.** Inside the undo window an op would be kept whole, with the chunk versions it bumped. Past the window it would collapse to a digest: who, when, which chunks, how many cells.
- **Chunk version retention would be the real knob**: undo depth, and how far back a volume rolls, bounded by how many versions of a chunk are kept, not by how many ops are.
- **Compaction would run in the world actor**, off the hot path, on a schedule.
- Measure before choosing the window: bytes per op, and bytes per stored chunk version after zstd.
