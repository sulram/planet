# 43. The sea's waves are band limited, and what is lost becomes roughness (decided)

Logged 2026-09-21.

The swell was three fixed scales (4 m, 1 m, 0.25 m) faded out past 400 m,
because from further each one fell inside a pixel and shimmered. So the sea
went flat from the sky, the sun's mirror became a perfect disc, and from the
sea floor the surface stopped swimming a few metres down, where the same fade
killed the ripple. The waves are now six octaves, 0.25 m to 256 m, each kept
only while a pixel can show it: the rule the generator already uses for
terrain (29), asked per pixel from the derivative of the position instead of
from a mesh spacing. There is always an octave in reach, so the sea never
shimmers and never goes flat. What a pixel can no longer show is not
discarded: the variance of those slopes widens the sun's specular lobe and
dims it in proportion, so the glint spreads into the road of light a real sea
has (the reasoning of LEAN and Toksvig normal filtering, on an analytic
spectrum instead of a mip chain). Refraction through the surface became an
angle rather than a displacement capped at 6 m, so what is far swims as much
as what is near. Value noise gained an analytic slope (`value_noise_slope`),
one lookup where a finite difference took three: with the dead octaves
skipped, a water pixel now costs about what it did. Rejected: stretching the
old fade further (the fine scales shimmer again); normal maps with mip chains
(texture files, and the look is procedural first); an FFT ocean or Gerstner
waves in the vertex (a system of their own, and the water mesh is coarse
where the problem is).
