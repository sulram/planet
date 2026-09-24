# 36. One scene target, a chain of stages, one tone map (decided)

Logged 2026-09-21.

Every scene shader used to tone map and encode its own output, straight into
the presented target. Nothing could come after: bloom needs light above white,
clouds need the scene's depth, and water blended over already curved colour.
The world is now drawn into an HDR target in linear light, its depth kept, and
a compositor runs full screen stages over it, ending in `output` (exposure,
filmic curve, encoding). An effect is a WGSL fragment entry and a line in the
chain, in the manner of a post process stack. Cost: two half float targets per
view (15 MB at 1280x720) and one full screen pass. Rejected: tone mapping per
shader with a separate bright pass for bloom (two truths about exposure), and
a generic pass graph (no second consumer yet).
