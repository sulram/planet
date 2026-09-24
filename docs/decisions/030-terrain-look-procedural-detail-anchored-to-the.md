# 30. Terrain look: procedural detail anchored to the planet (proposed)

Logged 2026-09-20.

From docs/TERRAIN_RENDER_BRIEF.md, deliveries 1 and 2, without texture files
yet: gloss is explicit in the vertex contract, rock is exposed per pixel from
the slope (the same at every LOD), and three scales of value noise shade and
bump the ground. The noise lattice repeats every 1024 m and each patch passes
its origin wrapped to that period in f64, so detail is fixed to the planet and
no large f32 position is ever built. Light gained a sky and ground ambient, a
filmic tone curve, starlight and moonlight. Textures with recorded provenance,
shadows and grass remain on the ROADMAP.
