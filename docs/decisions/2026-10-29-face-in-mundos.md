---
date: 2026-10-29
lives_in:
  - ARCHITECTURE.md#a-world-hosted
resolves:
  - what-a-planet-says-about-itself
---

# A planet's face in the catalog is set in mundos

**Decision:** mundos's catalog and door show a planet by its name, with the description and cover a superadmin sets in mundos (mundos's decision face-set-in-mundos). `GET /api/world` says what the world speaks and nothing of its face.

**Why:** mundos holds a description and a cover for every world, and a superadmin's word stands over what a world says of itself: a field inside planet would be overruled on the one screen that shows it.

**Rejected:** A title, description and cover set by builders inside planet and answered at `/api/world`: a second place for what mundos shows; it waits on the roadmap for builders who ask for it
