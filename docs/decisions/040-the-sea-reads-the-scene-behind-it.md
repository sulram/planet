# 40. The sea reads the scene behind it (decided)

Logged 2026-09-21.

The sea was a blended surface with one alpha: it could not tint the floor per
colour, bend it, or show anything from below but a flat ceiling, and under it
the whole world was fogged by the full distance, so a hill on the shore a
metre of water away vanished. It is now drawn after the opaque world and sky,
in its own pass, reading a copy of the picture and the depth: refraction and
per colour absorption over the water really crossed from above; from below,
the world above bent into Snell's window. Things past the surface are drawn
with air alone and the surface lays the water over them, so water is counted
once. Cost: one full target copy a frame and a pass without depth testing
(the shader discards behind nearer scenery). Rejected: a second render of the
scene for refraction (twice the terrain). From below, the physical bend was tried and dropped: it
folds the whole horizon into a cone overhead, so the shore hung in the sky,
the look toward it was a dark mirror, and a swimmer in front of the surface
left a ghost where their pixels were refused. The world above is now seen
straight through, rippled by the swell, a mirror only at a glancing look: a
swimmer sees the shore where it is. Only what lies past the surface along its
own ray is shown, read one texel at a time so no edge blends in.
