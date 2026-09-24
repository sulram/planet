# 34. Sun shadows in cascades, casters apart from the view (decided)

Logged 2026-09-21.

Sun shadows are three cascades around the eye (40 m, 400 m, 4 km), not one map
near the player. The first version had two, ending at 640 m: a mountain on the
horizon neither cast nor received, and at a low sun the far field stayed lit
while the near field was dark. The third cascade costs one more pass of coarse
patches; on the development machine the three together add 0.4 ms a frame
(native 1280x720, 0.95 to 1.37 ms).
Casters are chosen by the client from built ancestors, apart from the drawn
list, so an offscreen hill still shades the view and no generation is asked
for. Rejected: a horizon angle baked per vertex (a generator cost per sample,
and it cannot see avatars or builds).
Not done: a quality choice in the shells, which the Pi will need.
