# 51. A frame's budget counts chunks made, not meshes finished (decided)

Logged 2026-09-21.

The voxel streamer blew the frame budget in WASM and would not come down. Four
chunks a frame cost 20 ms against a budget of 12; one chunk a frame cost the
same 20 ms. That the number made no difference was the whole finding.

A mesh reads one cell past its chunk on every side, so a chunk at the frontier
pulls in the 27 around it. `warm` generated all 27 before returning, and a
budget checked between chunks could only ever stop on a multiple of that. The
granularity of the work was 27, and the budget was a fiction.

Now `warm` takes a limit, generates what it can and says whether the
neighbourhood is whole. A chunk that is not ready stays at the front of the
queue and the next frame carries on where this one stopped. The budget counts
chunks **generated**, because that is the work; counting meshes finished would
let one frame do twenty times another's.

Measured, `bun run bench`, ten second descent: worst frame 20.98 ms before,
3.75 ms after, median 1.89, none over budget. The walk through cave country
went from 20.23 ms to 4.57.

Three things were optimised before this was found, and none of them was the
fault: sorting the queue by a key worked out once instead of per comparison,
keeping a chunk's position beside it instead of folding a seam to get it, and
skipping the fold for border cells that never leave their sector. They are all
kept, because each is right, and none of them moved the number. What found it
was the bench printing which frame was worst and how many were over, and an
experiment with the budget set to zero.

Rejected: raising the frame budget, which is how a design stops being measured;
meshing without the border, which opens the seam 48 is about; a fixed cap on
chunks a frame with no relation to what a chunk costs, which is the tuned
constant that decision warns against.
