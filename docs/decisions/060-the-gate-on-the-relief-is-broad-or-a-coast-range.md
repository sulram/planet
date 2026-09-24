# 60. The gate on the relief is broad, or a coast range is a cliff (decided)

Logged 2026-09-21.

Mountains came out as a plateau with a wall: flat green, a vertical grey face,
a snow cap on top. Three explanations were tried and the first two were wrong,
which is worth recording because both were plausible and both were refuted by
measuring rather than by looking.

**Not the ridged noise.** `eroded` folds each octave toward a crease, and the
fold is constant from the third octave down, so every scale is ridged equally.
Fading the fold out with the wavelength is physically the right story, and it
did nothing: mean kink over a transect went 0.1088 m to 0.1064 m and the count
of local maxima went **up**, 207 to 230. Reverted, because a constant with no
fault to point at is a constant waiting to be tuned.

**Not the sea floor either.** The steepest grade anywhere on the transect, 555%,
is under water: a continental slope, not a mountain.

**It is `inland`.** On dry land the numbers are median 15.5%, p90 46.9%, p99
93.8%, max 160%. Decomposed at the steepest land sample, the terms fall
together: the constant base term drops 25% and the mountain term drops 26% over
the same 20 m. An identical relative drop means the mountain noise contributed
nothing and the common factor did all of it.

That factor was `smoothstep(0.0, 0.22, land)`, and `land` is the source
elevation over `LAND_M`. So the gate on the relief was a function of the very
elevation it scales: a steep source makes a steep gate, the gate multiplies a
thousand metres of relief, and the relief comes out amplifying the source's own
gradient. A positive feedback, and it lands exactly where a range meets the
sea.

**The cure was already in the file, and one channel had been left out of it.**
`lift` is read at 250 m and `ranges` at 1000 m, and the comment on those
footprints says why: *"Read at the texel, a coast range becomes a wall and a
foothill becomes a needle."* `inland` was the only broad channel still read at
the mesh's own resolution. It now comes off the belt sample, which is already
taken, so this costs nothing, times a fine factor that pins it to zero at the
shore the mesh actually draws. Near the shore the broad factor is itself near
zero, so the fine one's steepness never reaches the relief.

Measured, same transect: median 15.5% to 14.2%, p90 46.9% to 42.5%, p99 93.8%
to **65.8%**, max 160% to **88%**, and the peak of the range 759 m to 761 m.
The tail collapsed, the median barely moved, and the mountain kept its height.
88% is about 41 degrees, which is the angle of repose, so the ground no longer
stands anywhere loose rock could not.

The comment on `Shape::land` had the wrong reason written on it: *"the terms it
feeds all saturate, so a sharp coast is a narrow beach and never a wall."* They
do saturate in height. Saturating in height is not enough when what is gated is
a thousand metres, because the wall is in the slope.

No golden hash moved. `fingerprint` builds a recipe whose source is `generated`,
and the plates path keeps the old gate: its `land` is plate crust, which is
already broad, so it never had the feedback. The fault and the fix both belong
to the field path.

Rejected: a slope limiter over the finished height, which needs a relaxation
pass and cannot be per sample; strengthening the Quilez slope damping in
`eroded`, which is the knob that exists but works on the unit sphere gradient
and so cannot know about `relief_m`; widening the `0.22` ramp, which cannot
work at all, because `land` near that coast moves about 0.005 a metre and the
ramp would have to be ten land units wide to spread the relief over two
kilometres.
