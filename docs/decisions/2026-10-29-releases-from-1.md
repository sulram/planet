---
date: 2026-10-29
lives_in:
  - /release.config.js
  - /AGENTS.md#commits
supersedes:
  - 2026-09-20-021-versions-count-from-0-0-1-dev-publishes
---

# Releases count from 1.0.0, with Semantic Release's own rules

**Decision:** `dev` publishes `1.0.0-dev.N` and merging it into `main` publishes 1.0.0, planet's first release. The commit types bump as Semantic Release's defaults say: BREAKING CHANGE major, `feat` minor, `fix` and `perf` patch.

**Why:** The anchor tag `v0.0.0` never reached GitHub, so CI published `1.0.0-dev.1` to `dev.14`, and mundos runs those images on the box: a 0.0.x now would be an older version, which mundos never upgrades to. On a 1.x line the pre-1.0 rules guard nothing.

**Rejected:** Back to `0.0.1-dev.N`: it means deleting published tags and images that worlds run · keeping `feat` and `fix` as patches on 1.x: a version that no longer says what changed
