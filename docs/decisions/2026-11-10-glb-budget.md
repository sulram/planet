---
date: 2026-11-10
lives_in:
  - RENDER.md#_3d-models
---

# A 3D model's budget

**Decision:** A 3D model draws at most 65,536 triangles, a face seen from both sides counting twice; its meshes hold at most 196,608 vertices, each mesh once; its pictures hold at most 2048 × 2048 texels, each picture once; and it draws with at most 32 materials. The numbers live in `client::model3d` and again in the items panel's `budget.ts`, held equal by `bun run docs`. A model is weighed off its document and its pictures' headers, in the browser before the bytes leave and by every client when a file arrives; over any number it is refused with the reason, and the panel says the number.

**Why:** A model is any builder's upload and every visitor's engine draws it, a phone's or a Raspberry Pi's as a desktop's, whole at every distance with no coarser level: what one costs is bounded by its file, never by trust. 65,536 triangles is a detailed figure or a small building, what a phone draws of one model among many at sixty frames a second. 2048 × 2048 is 16 MiB as RGBA, 21 with its mips, what a screen shows of a model a few metres away. 32 materials is 32 draws in the solid pass and in each shadow cascade it reaches. The vertices are three corners a triangle: a file whose vertices its triangles use is never over it, and it bounds the reading no other number does. Lowering a number later refuses models a world already holds; raising one is cheap, so the numbers start low.

**Rejected:** The 20 MB file cap alone: a file that size holds millions of triangles · a budget per world or per level: a visitor's device is the same whoever uploaded · simplifying an over-budget model on arrival (decimation, smaller pictures): our own transcoding, which planet does not add; the builder's tool does it better
