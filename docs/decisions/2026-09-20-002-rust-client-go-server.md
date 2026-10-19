---
date: 2026-09-20
---

# Rust client, Go server

**Decision:** Marlus wants to learn both. Rust + wgpu gives one client for desktop, browser
(WASM), Pi and headsets. Go fits an authoritative network server and embeds
PocketBase. One protocol schema generates both sides.
