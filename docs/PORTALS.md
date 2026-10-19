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

- `Shape::Portal(to)` in `crates/part`: a standing ring two metres across,
  a rim in the colour in hand round a pane, leading to a landmark by the
  name the world gave it. `None` leads nowhere: a landmark never chosen,
  or gone.
- The rim shines with its own light whatever the paint's finish and is a
  lamp, as a landmark's star is, so a portal is found at night. The panel
  offers its colour alone.
- Placed by the items plugin as a picture is: in front of the body, facing
  it, not solid. The panel lists the chart to pick where it leads;
  `items.take { item: 'portal', to }` and `items.set { to }` on the seam.
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

## The view through it

- A portal's pane is a window on the world past it, drawn as Valve's
  Portal draws its portals: the view's camera is carried from the ring's
  frame to the landmark's, as a body that walked through would be, and the
  world is drawn from there first, into a slot of its own at a quarter of
  the view's height (`render::panes`, `scene::Through`).
- The eye past the portal keeps the view's own projection, so a point of
  the pane and the point seen through it fall on the same pixel: the pane
  samples that picture where it stands on the screen, and the window has
  parallax. The picture is HDR, before bloom and the tone map, which run
  once over the view with the pane in it.
- The world past a portal is its ground, sea, sky and clouds alone: no
  builds, no bodies, no shadows, no grass. The streamer splits for the
  nearest of two eyes, the one past the portal four times as far as the
  view's, so a quarter height sight costs a quarter (`client::terrain`).
- One portal is seen through at a time: the nearest within 32 m whose
  landmark is known here, held or charted. Every other pane is dark, lit
  with its own faint light. A pane casts no shadow: light passes a portal.
- What is drawn ahead and not built: the builds seen through it, the
  oblique near plane, the wobble (sketches/portals.md).
