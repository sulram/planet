# 53. Nothing comes down before its replacement is up (decided)

Logged 2026-09-21.

Crossing a level boundary opened a hole. The streamer dropped a chunk the
moment the pyramid stopped wanting it and queued the chunk that replaces it,
and the queue is drained a few chunks a frame: measured, the ground under the
eye was missing for 37 frames of a descent.

A stale chunk now stays drawn and joins a retiring set. It comes down when the
ground it covered is drawn at the level that took over: its parent when the eye
moved away, its eight children when it moved closer, because the shells that
meet are always one level apart. When neither is coming - the band moved, the
world was left behind - the queue running dry says so.

The cost is holding both for as long as the handover takes, which is the price
of the handover being invisible and is the right way round.

What makes this stay fixed is a test rather than a screenshot: a descent from
three radii to the ground, a step at a time, with the frame budget a real frame
has, asserting the ground under the eye is drawn at some level the whole way.
It reports 37 missing frames against the old behaviour and none against this
one. A picture cannot see a hole that lasts half a second.

Rejected: building the replacement before removing, synchronously, which is the
stall the budget exists to prevent; a fade between levels, which hides a
handover rather than ordering it and needs the renderer to know about levels.
