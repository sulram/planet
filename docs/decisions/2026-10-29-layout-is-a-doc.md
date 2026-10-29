---
date: 2026-10-29
lives_in:
  - LAYOUT.md
  - /AGENTS.md
amends:
  - 2026-09-24-068-the-root-guide-holds-only-what-applies
---

# The root guide keeps to kb's word budget; the layout is a doc of its own

**Decision:** The project's part of AGENTS.md stays under kb's `agents.budget`, the default 1500 words: rules, one line each, linking the doc that explains them. The crates, the folders and the commands live in LAYOUT.md, which `bun run docs` holds to the tree: every crate, plugin and script named.

**Why:** The guide enters every session's context, and the layout is a description of the tree, read when a session needs it.

**Rejected:** Raising `agents.budget` in kb.config.ts: the guide grows back, and every session pays for it · the layout in ARCHITECTURE.md, already at its 200 lines
