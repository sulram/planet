# 31. The moon is a place, and the first gravity field (decided)

Logged 2026-09-20.

Marlus wanted to fly to the moon, find it bigger, be pulled upright by it and
walk on it with a higher jump. The moon is now a sphere with a real position
(an orbit on rails, a function of the clock) and a real radius, drawn by a ray
and sphere test in the sky pass, so it grows as you approach and shows correct
phases from anywhere. The controller knows which body holds the avatar: inside
the moon's field (a sphere with a range, as decision 19 describes) the position
is stored relative to the moon, so the avatar rides its orbit for free, down is
the moon's centre, gravity is a fifth, and the shown up eases round instead of
snapping. Leaving takes a little more height than entering (hysteresis). The
moon is a smooth ball for now: terrain on it needs the ball topology, which
stays on the ROADMAP. The atmosphere shell doubled to 5 km with a thinner
density, by Marlus's eye. Rejected: keeping the moon as a painted disc with a
fake "arrival" trigger.
