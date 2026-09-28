# 76. Building starts in the client: a volume opens where you stand, and a stroke is a click and a drag (decided)

Logged 2026-09-28.

The first builder lives in the client alone, on the web and the desktop.
Taking a tool where no volume stands opens one around the body, 64 cells a
side and as tall, its floor at the ground under the feet. Three tools, create,
delete and paint, and a palette of sixteen colours. A stroke is one click and
drag: it lies as a slab on the side it started on, or, with Alt held as it
starts, stands up from that side as a wall facing the eye; the pointer is read
on the surface the stroke shows. A click is one cell. What a stroke would
change shows as a ghost. Escape drops a stroke half drawn, then the tool;
every stroke that lands can be taken back and put back. Building, the pointer
is free: the primary button is the tool's and the secondary one looks. A body
on foot stands on the cells and stops at a wall; in flight it does too, unless
a tool is in hand.

**Why local first.** Marlus wanted Cryptovoxels' builder, create, delete and
paint by click and drag, seen working before anything is kept. Sending a
stroke is the op of M3 and who may build where is M4, so nothing here is sent
or stored, and a reload forgets it. Opening a volume where you stand is the
seed of drawing one with a gizmo (M4). Its floor is the ground under the feet
as it is, which Marlus confirmed.

**Why a slab unless Alt.** A first version read the stroke's way from its
first movement: out of the side on the view, a wall. With the camera behind
the body, forward along the ground and up into the air go the same way on the
view, and slabs became impossible to lay away from the eye. Marlus chose an
explicit key. Shift and Control are run and descend, so the tools are keys,
1 2 3 and the panel, and the wall is Alt.

**Why the pointer is read on the surface.** Read in the middle of the layer
of cells, half a cell above the side clicked, the cell under a low pointer is
one to three cells away from the one it shows, and a click laid a row.
Cryptovoxels reads the end of a drag off whatever mesh is under the pointer,
so a drag over the sky ends nowhere and one that leaves the plane fills a box
its preview never showed (`refs/retro`, read for ideas). Here a stroke holds
its layer whatever is behind it, and its ghost is exactly the cells it will
change.

**Why a free pointer.** The third person camera looks at the head, so the
middle of the view is the body. A free pointer aims past it, and a phone has
no captured pointer to aim with.

**Why undo is local.** Each stroke keeps its box before and after, at most a
hundred strokes. It is the undo window of M3's two tiered log, taken before
there is a log.

**Why flight collides.** A builder flies through what they build; anyone else
flying into a house should meet its walls. Open ground holds a flyer up and
never stops it, as before.

**Rejected:** sending strokes now, M3 whole before a cube was seen; reading a
stroke's way from its first movement; modifiers for tools as Cryptovoxels has;
reading the end of a drag off the mesh under the pointer; aiming through the
middle of the view; volumes anywhere in the build band (58); flight through
cells always.

**Lives in:** WORLD.md § Two layers, ARCHITECTURE.md § Clients and UI,
RENDER.md, `crates/client/src/build.rs`, `apps/web/src/lib/engine/Build.svelte`,
`crates/ui-native`.
