---
date: 2026-10-19
lives_in:
  - PLUGINS.md#the-core-s-words
  - GLOSSARY.md
  - ROADMAP.md#the-parts-111
  - sketches/portals.md
  - /crates/part
  - /crates/seat/src/lib.rs
  - /crates/world/src/parts.rs
  - /crates/client/src/parts.rs
  - /crates/client/src/lib.rs
  - /proto/planet/parts/v1/parts.proto
  - /plugins/items
  - /apps/web/src/lib/engine/Labels.svelte
---

# A landmark is a shape of part with a name, and the world charts them

**Decision:** A landmark is a part whose shape is a five pointed star bearing a name
(`Shape::Landmark(Name)`): a metre across, lying where it is placed, in the
colour in hand, lit with its own light whatever its paint's finish, a lamp, so
it is seen at night. The parts owner answers `chart` to anyone with every
landmark of the world, the nearest first, and each is said with the place
code of its anchor: a person, a link and an agent travel to it through the one
`go_to`. The core hangs its name over the star as it hangs a nametag over a
head, and the items menu offers it beside the cube, with the chart under a
star icon that every level sees.

**Why a shape of part.** Pose, paint, solid, shine, reach, gizmo, ops, store,
undo and permission are the part's, written once (111). A landmark adds one
record field and one mesh; nothing else of it is new.

**Why the name is bounded and inline.** `Name` is up to 64 bytes of UTF-8,
trimmed, with no control character, kept inline in the record, so `Shape` and
`Part` stay plain data every holder copies freely, and `part_from_wire` is
the one place a name is judged, on both sides. A `String` in the shape would
make `Part` `Clone` across twenty sites and allocate on every set of a drag
for a record that is otherwise a few numbers.

**Why always a lamp.** A landmark exists to be found, and a star that
disappears at night marks nothing. Its finish is Light however it is painted;
only its colour, its glow and its reach are the hand's.

**Why the world charts.** A client holds the parts near it alone; a landmark
8 km away is one it does not hold and must still travel to. The chart is a
question of the parts, at any level, because travelling is everyone's and
placing a builder's.

**Why a place code.** The chart says where a landmark stands as the code
`go_to` takes, so the front end travels as it does from a shared link or
chat's `@here`, with the veil and the address bar in step, and an agent asks
the same two things a person's hand does (94). Arriving is standing on the
star.

**Rejected:** a `String` name; a landmarks plugin of its own, since a landmark
is a part and the chart a question of the parts; the near list widened to the
world, which is a list to pick by, not to travel by; the name drawn on the
GPU, where there is no text, when the nametags' path is there; a portal
before a landmark, since a portal needs an address and the star is one.
