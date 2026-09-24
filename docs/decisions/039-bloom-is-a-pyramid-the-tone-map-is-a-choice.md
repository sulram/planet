# 39. Bloom is a pyramid; the tone map is a choice (decided)

Logged 2026-09-21.

Bloom is the first effect the compositor was built for. Light over a threshold
is halved down five levels and summed back up (the Call of Duty: Advanced
Warfare filter): thirteen taps down with a brightness weighted first halving,
so one blazing pixel cannot flicker the glow, a tent up. Wide glow, small
kernels, all at half size or less. Rejected: a separable Gaussian at full size
(costlier for a narrower glow). It runs after the clouds.
The tone map became a setting rather than a verdict: ACES stays the default
for its contrast, AgX is there because ACES skews bright hues (a sunset goes
yellow), Khronos neutral keeps authored colours for builders judging a
material, Reinhard and linear are references. One curve would have been a
taste imposed; five cost a switch in one shader.
