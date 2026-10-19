---
date: 2026-09-20
---

# Hybrid voxels: smooth terrain, cubic construction

**Decision:** Smooth terrain (density + surface nets) is what vehicles need and what the
generator's noise already is. Cubes keep crisp Cryptovoxels architecture.
Rejected: cubes only (vehicles flip on stairs, blocky LOD from orbit), smooth
only (no straight walls). Cost accepted: two meshers, two tool sets.
References: Roblox terrain + parts, Space Engineers.
