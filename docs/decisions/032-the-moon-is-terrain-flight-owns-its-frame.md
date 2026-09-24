# 32. The moon is terrain; flight owns its frame; gravity is continuous (decided)

Logged 2026-09-21.

Testing decision 31, Marlus found that the far side of the moon hid the stars
but not the sun or the planet, that the moon had no craters, and that the
approach jolted. All three came from shortcuts. The painted sphere lived in
the sky pass, behind everything with depth. The moon is now a second body on
the same terrain quadtree: `Body::Planet` and `Body::Moon` differ in radius,
depth and having a sea, patches are built around their body's centre, and the
renderer adds the centre of the moving body each frame. A crater generator
(stacked cell grids of bowls with rims, footprint filtered like the planet)
joins generator v2, guarded by its own golden hash. Depth does the occlusion;
sunlight is what neither sphere shadows, which gives night and eclipses alike.

The jolts had two causes: the avatar only started riding the moon 6 km out,
while it sweeps past at 730 m/s, and down flipped in one frame. Marlus asked
that flight predominate. So three things are separate now. The frame of
reference (sphere of influence, 4 moon radii, with hysteresis) changes far
away where nothing shows. Gravity turns continuously from one body to the
other with distance. And the avatar has a frame of its own, `frame_up`, that
turns toward gravity by rotation, carrying view and facing with it: quickly on
foot, in flight only near a surface. Look at the planet from the moon and fly:
you go there, and nothing turns you on the way. Rejected: easing only the
drawn body while the controls snapped (what decision 31 shipped). The
atmosphere settled at 3.6 km. The avatar's `Body` type became `Figure`:
in the GLOSSARY a body is celestial.
