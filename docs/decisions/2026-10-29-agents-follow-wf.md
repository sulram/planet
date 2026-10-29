---
date: 2026-10-29
lives_in:
  - /wf.config.ts
  - /AGENTS.md#parallel-agents-wf
---

# Parallel agents follow @tekne/wf; planet keeps only the preparation and the gates

**Decision:** The flow of parallel agents is @tekne/wf's: one worktree, branch and agent per task in a herdr pane, opened by `bun wf spawn` and delivered to `dev` by `bun wf ship`; its rules are the `wf gen` block in AGENTS.md and `bun wf --skill`. `wf.config.ts` holds what is planet's: `bun install` to prepare a worktree, `bun run docs`, `bun wf check` and `cargo fmt --check` as the cheap gates, `bun run check` as the expensive one.

**Why:** The studio's projects run one flow, kept in one place: marlus.com and itsrio-plataforma already do, and kb 0.6 leaves worktrees to wf. The hand flow asked each session to merge, close and clean up from memory, with no gate before `dev`.

**Rejected:** kb's worktrees module with the herdr flow written in AGENTS.md: a rule with no tool, every merge and close by hand · `bun run check` as a cheap gate: minutes of Rust, WASM and Go each time an agent checks its work, where the ship runs it once
