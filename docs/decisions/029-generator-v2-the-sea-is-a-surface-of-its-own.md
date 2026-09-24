# 29. Generator v2; the sea is a surface of its own (decided)

Logged 2026-09-20.

Marlus found v1's mountains needle like and wanted a sea to swim and dive in.
v1 stays frozen for the worlds that use it; new worlds get v2: massifs from a
slope damped fractal sum (after Quilez) whose first octaves stay smooth, a sea
floor with shelf, plain and seamounts, and sampling filtered by the footprint
of the mesh that asks, so coarse patches neither alias nor pop. Collision and
anything saved sample at full detail. The terrain mesh now carries the real
ground, and each patch that dips under sea level carries a water surface too:
blended, double sided, coloured by per channel absorption, Fresnel to the sky,
Snell's window from below. Underwater is a medium, not a fog colour: red dies
first. Rejected: tinting the terrain mesh blue (v1's look, not penetrable), and
a single planet sized water sphere (no depth per vertex, no shoreline). Next
step, on the ROADMAP: refraction from the scene depth and colour.
