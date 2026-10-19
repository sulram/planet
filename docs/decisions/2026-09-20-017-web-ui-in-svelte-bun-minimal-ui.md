---
date: 2026-09-20
---

# Web UI in Svelte + Bun; minimal UI elsewhere

**Decision:** Builder and player modes are full in the browser. Pi, headsets and the native
executable get a simplified UI. Consequence: tool logic lives in Rust behind a
command/event seam so every client shares it.
