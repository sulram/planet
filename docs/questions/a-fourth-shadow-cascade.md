---
group: marlus
owner: Marlus
unblocks: shadows of builds seen from further than 35 m; the contact shadow of an avatar
since: 2026-09-30
---

# A fourth shadow cascade

Three step tenfold (40 m, 400 m, 4 km): a texel is 7.8 cm near the eye and 78 cm from 35 m on, wider than a cube. Tried and put back: four at 16, 100, 630 and 4000 m, three constants in `render::shadow`. The shadow of an avatar gets its legs and meets its feet; a texel is 3.1 cm near the eye and 20 cm out to 100 m, and 1.2 m from there to 630 m against 78 cm with three. It costs one more pass of casters. The one reading of that cost, 0.2 ms a frame on the development machine, is taken beside a browser running the world and is no measure: measure alone first, and the numbers go in the commit

**Context:** ROADMAP wishes · DECISIONS 34, 83.
