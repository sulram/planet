# 41. The shore is painted per pixel, by height (decided)

Logged 2026-09-21.

Far coasts showed teeth, worse with distance. The ground was already smooth;
the color was not: sand was a class decided per vertex and interpolated, so
its edge ran along the diagonals of the mesh, a kilometre a tooth at the
coarsest level, and a band eight metres tall was caught by a vertex only by
luck. Sand and sea floor are now decided in the terrain shader from the
height of the pixel over the sea, as rock already is from the slope: the line
is the contour of the ground, smooth at every LOD, wandered by the anchored
detail up close and softened by the pixel's own reach from afar. It costs no
vertex data. A vertex under the shore carries the cover of the highest land
beside it, so no beach bleeds uphill across a coarse triangle. The generator
is untouched: its materials are hashed and frozen. The shader holds the shore
heights of generator v2 (2 m, -6 m); a v1 world gets the same beach. Rejected:
a blur after the fact (softens everything, leaves the teeth their shape);
continuous cover fields in the vertex for snow and forest too (the same cure,
a new vertex contract: ROADMAP).
