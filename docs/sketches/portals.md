# Portals: a door from one place to another

Drawn 2026-10-19, with the landmark placed (116). The ring and the
crossing stand in the items plugin: `Shape::Portal(to)`, a pane that leads
to a landmark by the name the world gave it. Nothing else here is decided.
It is the whole a portal is measured against: what it stands on today, what
the renderer would gain, and the order a campaign would pull it in.

## The thread: a portal is a part that leads to a landmark

A portal would be a shape of part, `Portal { to }`, naming a landmark by the
name the world gave it. The landmark is the address and the portal the door:
there is no destination to type, no pair to keep in step, and a landmark
removed leaves a portal that leads nowhere and says so. A standing ring two
metres across by default, not solid, placed as a picture is, in
front of the body; its panel row lists the chart to pick the destination from.
The way back is a second portal placed at the destination; a button "place the
way back" would place it in one pair of ops, standing on the star and facing
the way one arrives.

The star's own rotation would be the arrival heading: one lands on the star
facing the way its point toward `+y` points. A landmark turned by its gizmo is
then a landmark that says which way to look on arrival, with no field added.

## The view through it

A second eye standing on the destination's star, facing its heading, drawn at
a quarter of the screen's height every other frame into a target the portal's
quad samples as a picture (`PictureDraw` with `shine` 1, so it reads as a
screen and is not lit twice). What the renderer has: `Renderer::render`
takes any number of views of one `Frame`, camera relative each, built for a
headset's two eyes; a headless path draws into a texture and reads it back;
every point of every body is drawable from anywhere, since the six roots of
the far field are always built. What it lacks, and would gain in this order:

- A frame per view, or `render_into(frame, view, slot)`: today every view
  shares one patch list, selected and culled for the main eye, so a view from
  elsewhere sees the ground the main eye streamed and nothing nearer.
- A render target that is a `PictureId`: HDR, so the one tone map runs once,
  over the main scene with the portal in it. The picture pipeline takes
  uploaded `Rgba8UnormSrgb` images alone today.
- `Terrain::select` by the nearest of two eyes, with a share of the six builds
  a frame, and its own looser split so a quarter height view costs a quarter.
- A `look` from the destination too, or the portal shows bare nature: the
  server answers what is near the body's stance, and nothing of the cells or
  the parts 8 km away reaches a client.

The first cut would be the far field alone through the portal, a view of
mountains and sea from where the star stands, which costs one patch list and
no server change; the builds through it come after. One portal view at a
time, the nearest in view within 32 m, none past it, and the portal drawn as
a dark pane with its rim when no view is given.

## The crossing

Built: in the plugin's turn, after the body moved, the feet's step passes
the pane where the body, feet to head, meets the ring's circle, from
either side, and the body goes through
`Host::go_to` to the landmark's place with the star's bearing. A leap arms
the crossing again only once the body stands clear of every pane, so a
return ring on the star sends nobody straight back. What would follow: the
body keeping its velocity, so a run through a door is a run out of the
other; a flag in the stance for a leap, so another client snaps the figure
instead of sliding it 67 ms through the ground as it does now; the veil
lowered for a leap whose ground is not built.

## The transition

One full screen stage in `Composer::finish`, between the bloom lay and the
output: the picture's UVs displaced by a damped radial sine, the wobble, driven
by the clock already in the view's uniform and an amount in a new field of
`Frame`, set by the client at the leap, 1 to 0 over 0.8 s, the pass skipped at
0. The settings' `Effects` are the person's and saved; the transition is the
frame's. Later, the portal quad's own shader: a rim that shimmers, and the
view refracted at the edge as the sea refracts what is behind it. The veil the
web page lowers while the ground streams in would stay for a leap across the
world and lift at once for a portal whose destination is already built.

## Agents and the seam

`items.take { item: 'portal', to }` places one; the crossing asks `go_to` as a
link does. Nothing a hand does an agent cannot ask by its parameters (94), and
a headless client walks through a portal as a person does.

## Costs, measured before chosen

The second view in WASM against the frame budget, in `bun run bench`, with
the far field alone and with builds; what a quarter height view costs on a
Pi; whether every other frame reads as a view or as a slideshow, tried by
Marlus in the world.

## What would prove it wrong

- A second eye's streaming starving the main one. Then the portal shows a
  still picture, taken when a builder stood at the landmark and kept as an
  asset, refreshed on a cadence.
- A destination with no builds within reach of the world's `look`. Then the
  server answers a `look` from a second stance, the portal's, as a room of
  its own.
- A portal wanted to another world (VISION). Then `to` names a world and a
  landmark in it, and the crossing is the door's: mundos says where the world
  is, and the view through it is the still picture.
