# 17. Web UI in Svelte + Bun; minimal UI elsewhere (decided)

Logged 2026-09-20.

Builder and player modes are full in the browser. Pi, headsets and the native
executable get a simplified UI. Consequence: tool logic lives in Rust behind a
command/event seam so every client shares it.
