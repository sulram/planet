# Entities: things placed in a world

Drawn 2026-10-19, with the cells kept and building a plugin, while a cube is
being cut as the first placed thing on a branch beside this one. Nothing here
is decided or built. It is the whole that a cube, a `.vox` model, a GLB and
the frames and lights after them are measured against, so that each is one
more shape of the same thing and not a system of its own.

## The thread: a part is a placed thing

The world has two layers today: nature, a surface nobody edits, and the
cells, which are the address itself. A third kind of thing is coming from two
sides at once: Marlus asks for a cube one can move, turn and scale, and for a
`.vox` file one can upload and place with a collision switch. Cryptovoxels
has both as features of a parcel; Hyperfy has them as entities with a GLB.

The drawing: one noun, a **part**. A part has a pose, a scale, whether it is
solid to a body, and a shape. The shape is what differs: a cube is one voxel
of one paint; a **model** is a `.vox` file by the hash of its content; a GLB,
an image in a frame, a light, a portal and a spawn point would be shapes
after them. Everything else, the gizmo, the panel, the ops, the store, the
permission, the budget, is written once over the part and never over a shape.

By 108's two questions a part is the core's: with building switched off what
was placed must still stand and hold a body up, and land will give permission
over it as it does over cells. So parts would be a system, an owner under the
envelope beside `cells`, and building the plugin that places them.

## Why a model is a part and not cells

A `.vox` could be stamped into the cells as one gesture, and then collision,
storage and the picture would come for nothing. It would lose the rest:
MagicaVoxel paints with 255 colours against the world's sixteen (103), its
voxels are finer than a cell, a model is moved whole and turned by any angle,
and the file is what the person made and wants to see as they made it. A
model keeps its own palette, baked into its vertices, since 255 colours of a
mesh need no palette on the GPU.

## The pose

Where a part stands is said as the cells say where they are: a seat, and a
point of it in fractional blocks with a height, plus a rotation in the local
tangent frame, up being the body's up there. A part is kept by its seat with
an id of its own, never inside a volume, so a part on the line between two
volumes is one part and moving it from one to the next is a move like any
other. Permission is asked for every plot the part's box covers.

A part is rigid in its anchor's frame, where cells are bent corner by corner
onto the body (WORLD.md). A volume is 32 m wide and continuous with its
neighbours, so it must bend; a part is the size of a thing, and at 20.9 km of
radius a 10 m part sags 2.5 mm from the sphere, less than any cell. A part
larger than a plot would be refused, which keeps that true.

Scale is metres a voxel, one number or three. The panel would offer snaps:
a voxel to a cell, to half, a quarter, an eighth and a sixteenth of one. The
default worth trying first is an eighth, 6.25 cm, where a model of 32 voxels
stands 2 m tall: the size of a thing in a room. Rotation is free, with snaps
at a quarter turn and at fifteen degrees; nothing in collision or the picture
is special to a right angle, so the snap is the panel's.

## What a body collides with

A cube is an oriented box. A model is its own grid of voxels, read through
the inverse of its pose: a column under the feet is a line through the
model's grid, and the footing is the top of the solid at or under the feet
along it, found by stepping the line through the grid or by probing at half a
cell as the ground is probed (47). Solid off, and a body passes through: the
part is scenery. Whatever a part answers is taken together with the ground
and the cells by `Footing::with`, which already joins two floors.

This is where a `.vox` pays for itself before a GLB does: a model's collision
is exact and costs nothing to author, where OPEN still asks what a body
collides with inside a GLB shell. The cheaper answer there, a voxelizer at
import, would make a GLB a model with a skin, and the same footing serves.

## Files

Files are the server's, as 89 and ARCHITECTURE § Files draw them, and no
plugin's. A session that may build sends a file to `POST /api/files` with its
key; the world refuses what is not a kind it takes, `.vox` first, GLB and
images after, and what is over a size, names the file `<sha256>.<ext>`, and
keeps it in the bucket folder or, alone, in the world folder. `GET
/files/<hash>.<ext>` serves it, kept by a browser for good since the name is
the content. `GET /api/files` says what the world holds, for a library panel.
The kind is read from the bytes and never from a name, the cap is 20 MB a
file, and the answer is the asset's name alone: what a picture measures, the
owner's client knows before it sends.
A part's record names its file `asset://<hash>.vox`, and on start a world
deletes from its bucket folder what no part names (89). A file sent twice is
one file. The terms of use of what is uploaded are OPEN's.

The drop is the way in: a `.vox` dropped on the page is sent, and once named
it is placed where the pointer was, as Cryptovoxels does. The library panel
is the second way, for a file already held.

## The picture

A model is meshed once, when its file arrives in a client: every side of a
voxel that looks at air is a quad with the paint's colour and how open each
corner is, as the cells are meshed, so a model and a volume shade alike. A
cube is a model of one voxel. One mesh of a model is drawn once for every
part that names it, each with its own transform, camera relative as a
skinned mesh is. So the scene would carry a model mesh with an id, and in
each frame the parts to draw: a mesh and a transform each. The mesh's vertex
is the one the cells already have.

MagicaVoxel's materials would come later, as finishes of the cells came:
`_emit` as a light, `_glass` as glass. A part casts a shadow as a volume
does. From afar a part would be a box of its mean colour, then nothing: what
a frame affords from 256 m on is the budget's to say, measured.

## Images

An image is a part whose shape is a picture: a quad of the picture's aspect,
one metre high by default, with a texture, single or double sided, casting a
shadow. A knob says how much it shines with its own light, 0 a painting lit
by the sun, 1 a screen; video defaults to 1. The record names what the
picture is at every distance and nothing else: its pixel size, and the three
assets the owner's client made at placement, the original, a low version at
most 1024 px on its long side and a thumb at most 128 px, both WebP
(ARCHITECTURE § Files). Far draws the thumb, mid the low version, near and in
view the original: the level is picked by the picture's height on the screen
in texels against each version's, with hysteresis, so a step never thrashes.
A level is asked for over the asset seam when first needed and dropped when
the part leaves reach, under a texture budget that drops the longest unseen
first. Decoding is the shell's, as fetching is: the browser's own decoder on
the web, a codec crate on desktop, so the engine's WASM carries no codec and
the client is handed pixels. A video is the same record with a poster for its
thumb, plays only near and in view, under a decoder budget, and `VideoSource`
is one seam with a backend for each platform.

## Budgets

Cryptovoxels allows a parcel 1000 cubes and 500 models, and refuses a model
over 128 voxels a side. The drawing: a count of parts a volume may hold and a
count of voxels drawn, both the world's numbers in the recipe's manner, and a
model's size refused at upload, since a model too large for one part is a
build and the cells are there for it.

## One gizmo, one panel

Marlus: the gizmos, the controls and the panel must be the same for a cube
and a model, thought together. So there is one gizmo, the part's, in the
build plugin: handles to move along the two axes of the seat and up, to turn
about them, to scale whole or by axis, with the snaps, and the same panel
with numbers one scrubs, a shared control of the design system for a vector.
A shape adds its own rows to it: a cube its paint, a model its file and how
many voxels it has. Each is a command at the seam by its parameters (94):
place a shape at a pose, move a part to a pose and scale, set it solid or
not, remove it, so an agent places a model by its hash with no gizmo.

## An order, if a campaign pulled it

1. The reading of a `.vox`: parse, flatten its scene graph, mesh with
   occlusion, in a crate of its own with no sphere and no IO.
2. The upload path and the drop, and the file served under its hash.
3. Parts as a system, cut by the cube: the pose, the store, the ops, the
   footing of an oriented box, the gizmo. On the branch beside this one.
4. The model as the second shape: fetched, meshed, drawn with the parts'
   transforms, placed by the same gizmo.
5. The footing of a model through its pose.
6. Budgets and what is drawn from afar, measured in WASM.
7. A GLB as the third shape, with OPEN's question about its collision answered
   by a voxelizer or not.

## What would prove it wrong

- A model's grid read through a free rotation too slow in WASM for a frame
  with many parts near. Then rotation snaps to quarter turns in collision,
  and the picture keeps any angle.
- Parts kept by seat growing past what a client reads at once. Then an index
  by plot beside the store, as the cells have.
- A palette baked in vertices too heavy for a world of many large models.
  Then a palette texture a model indexes, one pipeline still.
- A `.vox` the size of a house uploaded again and again. Then the budget
  before the parser, and a model that is a build is refused with the cells
  pointed at.
