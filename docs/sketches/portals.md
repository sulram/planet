# Portals: a door from one place to another

Drawn 2026-10-19, with the landmark placed (116). The ring, the crossing
and the view through it stand: `Shape::Portal(to)`, a pane that leads
to a landmark by the name the world gave it. Nothing else here is decided.
It is the whole a portal is measured against: what it stands on today, what
the renderer would gain, and the order a campaign would pull it in.

## The thread: a portal is a part that leads to a landmark

Decided (DECISIONS portal-is-a-ring): a portal is a shape of part,
`Portal(to)`, naming a landmark by the name the world gave it. The landmark
is the address and the portal the door: there is no destination to type, no
pair to keep in step, and a landmark removed leaves a portal that leads
nowhere. A standing ring two metres across, lit, not solid, placed as a
picture is, in front of the body; its panel row lists the chart to pick the
destination from. A portal is one way: the way back, when wanted, is a
second portal placed by hand at the destination.

The star's own rotation is the arrival heading: one lands on the star
facing the way its point toward `+y` points. A landmark turned by its gizmo is
then a landmark that says which way to look on arrival, with no field added.

## The view through it

Built, in part: a second eye standing where the camera would be past the
portal, the view's camera carried from the ring's frame to the star's,
drawn at a quarter of the screen's height into a target the pane samples
where it stands on the screen, with the view's own projection, so the
window has parallax (PORTALS.md). The picture is HDR, before the
one tone map. What the renderer gained for it: a sight of its own per draw
of the world, a patch list per eye, and the streamer splitting for the
nearest of two eyes, the one past the portal four times as far.

What would follow, in this order:

- The builds through it: the cells and the parts near the destination,
  which the server tells nobody 8 km away. A `look` from the destination
  too, a room of its own, answered as the body's is.
- An oblique near plane for the eye past the portal, Lengyel's, so the
  ground between that eye and the star's plane is clipped instead of
  closing the window when the star stands on a slope.
- Every other frame, and the pane holding the last picture between, when
  the frame budget asks for it.

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
