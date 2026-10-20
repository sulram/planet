---
title: Portals
group: Shape
summary: "A portal: a lit ring that leads one way to a landmark, the walk through it, and the window its pane is"
order: 23
---

# PORTALS

A portal is a shape of part (DECISIONS portal-is-a-ring): what it is, how a
body passes it, and how its pane shows the world past it. What a part is
lives in WORLD.md; how the picture is made in RENDER.md.

## The ring

- `Shape::Portal(to)` in `crates/part`: a standing ring four metres across,
  a rim in the colour in hand round a pane, leading to a landmark by the
  name the world gave it. `None` leads nowhere: a landmark never chosen,
  or gone.
- The rim shines with its own light whatever the paint's finish and is a
  lamp, as a landmark's star is, so a portal is found at night. Its width
  is the hand's, of the ring's width: none at 0, the pane alone and no
  lamp; the middle at most. The panel offers its colour and its width.
- Placed by the items plugin as a picture is: in front of the body, facing
  it, not solid. The panel lists the chart to pick where it leads;
  `items.take { item: 'portal', to, rim }` and `items.set { to, rim }` on
  the seam, `rim` of the ring's width.
- A portal is one way. The way back, when wanted, is a second portal placed
  by hand at the destination.

## The crossing

- Read in the items plugin's turn, after the body moved
  (`plugins/items/client/src/cross.rs`): the feet's step from last turn to
  this one passes the pane's plane where the body, feet to head, meets the
  ring's circle, from either side.
- The body goes through the one `go_to` a link and the chart travel by
  (`Host::go_to`), to the landmark's place with the star's bearing: arriving
  is standing on the star, facing the way its point toward `+y` points.
- A landmark this client does not hold is found through the chart first;
  one the world no longer has leaves the body where it is, and the seam
  hears `items.crossed` with no place.
- A leap arms the crossing again only once the body stands clear of every
  pane, so a portal on the star one lands on sends nobody straight back.

## The passage

- A body that crosses falls into the portal for 0.35 s, in the falling
  gait, before it is through; it lands on the star and the picture takes
  0.65 s to come back: a second in all (`Passage` in `client`, `WARP_IN_S`
  and `WARP_OUT_S`).
- The picture is drawn into the portal and out of the body: a compositor
  stage (`warp.wgsl`) pulls the picture toward a point of the screen and
  streaks it along the pull, the colours parting a little, while a black
  hole opens there with a hot rim and the rest goes dark; then the same
  let go. The point is the pane's middle on the way in and the body's
  waist on the way out. Before the bloom, which takes the rim's light.
- The plugin asks `Host::go_through(place, portal)`; the core keeps the
  time and travels, as a link does, when the fall is done.

## The view through it

- A portal's pane is a window on the world past it as a wide lens in the
  portal sees it: a camera carried from the pane's middle to the landmark,
  12 m over the star, looking the way the eye looks through the pane's
  middle, 140 degrees high (`LENS_*` in `client`). The world is drawn from
  there first, into a slot of its own as big as the pane is seen, a
  quarter of the view at least and the whole of it when the disc fills
  it, in steps (`render::panes`, `scene::Through`). The streamer's detail
  for that eye follows the same size.
- The window bends what it shows toward its rim in a band that pulses, the
  colours parting there: the picture, drawn smaller than the view, meets
  the view through a band that is the portal's and not a seam.
- The pane shows that picture as a fisheye: its middle along the line,
  each point of the disc that line turned toward the point, the rim 65
  degrees off it, so a disc far off shows the far side whole. Walking
  round the portal turns the line, and the window keeps its parallax. The
  picture is HDR, before bloom and the tone map, which run
  once over the view with the pane in it.
- The world past a portal is its ground, sea, sky, clouds and cells: what
  is built there shows through the pane; no parts, no bodies, no shadows,
  no grass. The cells' `look` carries a second eye, the star the portal
  leads to, and the world answers for the nearer of the two, so the
  volumes near the star are held, meshed and drawn as the near ones are,
  and dropped when the portal is no longer seen through. The streamer
  splits for the nearest of two eyes, the one past the portal four times
  as far as the view's, so a quarter height sight costs a quarter
  (`client::terrain`).
- One portal is seen through at a time: the nearest within 96 m whose
  landmark is known here, held or charted. Every other pane is a lens:
  what is behind it, bent toward its middle and pulsing, as through thick
  glass, with a faint rim, so a portal is a portal from afar. A pane
  becomes a window from a lens, and back, over 0.6 s. The panes are drawn
  after the sea and the clouds over a copy of the picture, which the lens
  reads. A pane casts no shadow: light passes a portal.
- What is drawn ahead and not built: the parts seen through it, the
  oblique near plane (sketches/portals.md).
