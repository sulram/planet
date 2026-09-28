# RENDER

How the picture is made: bodies in it, the sea, the sky, the light and the
compositor. What the world *is* lives in WORLD.md.

## Avatars

- An avatar is a VRM, named by an **asset reference**: a path under the asset
  root (`avatars/Kyle.vrm`) or an absolute URL (a user's own upload, M5). The
  client never tells them apart; nothing addresses an avatar by index.
- `assets/manifest.json` is the config of the instance's default set:
  `default_avatar`, `avatars` on offer, `clips` per gait. Edited by hand.
- Which avatar a person wears: the user's default avatar (future
  `users.avatar`), else the visitor's earlier choice (cookie), else a random
  one from the offer, which becomes the choice. A failed load wears
  `default_avatar`; the box figure covers the time nothing is loaded.
- Clips are authored once on a Mixamo rig and retargeted at load to the VRM
  humanoid (crate `avatar`), so every avatar shares every clip. Gaits: idle,
  walk, run, jump, fall, fly. VRM 0.x, one skin, PNG textures for now.
- Asset seam: the client does no IO. It queues requests by reference, the
  platform shell fetches (disk on desktop, `fetch` in the browser) and answers.
- `V` wears the next avatar on offer. The engine reports `avatar_changed`; the
  web app keeps it as the visitor's choice (`POST /avatar`).

## Volumes

- Every side of a solid cell that faces air is one quad, never merged, and its
  corners are bent onto the body through the addresses they are
  (`client::build`): neighbours share corners exactly, and a volume curves
  with a small world (75). A chunk works out its column corners once.
- `render::volumes` draws them flat (`volume.wgsl`), the paint's sRGB as
  albedo, through `lit_occluded`: how shut in a corner is by the cells beside
  it, squared, takes away ambient light only, and the sun is the shadow
  map's. Volumes cast into the cascades and receive.
- The ghost is the cells a stroke would change, lifted 12 mm off what they
  cover and drawn after the sky, blended, tested against depth and writing
  none (`Surface::Ghost`). Red over what a delete takes (76).

## Sea, sky and light

- The terrain mesh is the real ground, sea floor included. A patch that dips
  under sea level also carries a water surface: same grid, same indices.
- Water is drawn last, blended, from both sides: per channel absorption by
  depth, Fresnel to the sky, foam at the shore, Snell's window from below.
- The sea's waves are six octaves of drifting noise, a hand wide to a quarter
  of a kilometre long, anchored to the planet like the ground's detail. Each
  is kept only while a pixel can show it, as the generator keeps an octave
  only while the mesh can: near, all of them; from the sky, the long swell
  alone. What a pixel can no longer show becomes roughness, which widens and
  dims the sun's mirror, so a spark at hand is a road of light from above.
  One noise lookup an octave, value and slope together.
- The sea is drawn after the opaque world and the sky, in a pass of its own:
  the picture so far is copied aside and the water reads that copy and the
  depth (`Composer::behind`). It tests depth itself, having none attached.
  The side of the sphere turned from the camera (the far sea, past the
  horizon) is discarded: nothing else would hide it behind the near one.
- From above it refracts what lies under it and colours it by the water
  actually crossed (down to the floor, back along the ray), so shallows go
  turquoise. It takes sun, cast shadows and cloud shade as the land does.
- A camera under sea level sees through water as a medium (red dies first).
  What lies past the surface is drawn with its air alone; the surface lays the
  water between, and shows the world above straight through, bent by the
  waves' slope as an angle (so a far ridge swims as much as a near one); it
  mirrors the sea only at a glancing look. Only what lies past
  the surface along its own ray may be seen through it.
- Clouds and sea are drawn nearer last: clouds first for a camera under the
  sea, after it for any other. `water_clarity` stretches a swimmer's sight.
- Swimming is part of walking: in water too deep to stand you float at chest
  depth, `Space` leaps, `C` or looking down while moving dives, idle drifts up.
- What holds a body up is a **footing** (`client::collision`): the top of the
  solid at or under its feet, and the bottom of the solid over its head. The
  ground out in the open; over a volume, its cells too (`client::build`),
  where a body is 1.2 cells wide and stands on the highest cell under any of
  it. No mesh: a footing costs 2 us, and a step asks for four (47).
- A rise of one block is taken in stride and two is a wall, to be jumped or
  flown, going up and coming down. A body needs its own height of room to walk
  into a place, unless it already has less, so a tight place is not a trap.
  Blocked, a step is tried along one address axis and then the other, which is
  what slides a body along a wall.
- Rock at knee height means the body is in rock rather than on it, and then
  the floor is the ground itself: a wall is not walked into, a buried body is
  let out upward, and nothing falls through the planet.
- Collision is the ground in full detail, never the filtered one: what a body
  stands on may not change with where the camera is. It reaches as deep as the
  field does, which is deeper than the volume is drawn.
- Inside the ground the third person boom is cut by the rock behind it instead
  of lifted over the terrain, so the camera stays in the cave with the body.
  Flight keeps its own floor over the drawn ground, except under the ground,
  where the footing takes over. With no tool in hand a volume's cells are
  floor, roof and walls to a flyer too; building, it flies through (76).
- Sky: a shell atmosphere (3.6 km), a sun, stars fixed to the world.
- Bodies: the planet and the moon share one terrain quadtree (`Body`). Patches
  are built around their body's centre; the renderer adds where the body is
  this frame. The moon orbits on rails, 160 km out, 8 km radius, craters from
  generator v2, no sea. Craters are searched in a cell grid per sample; the
  few basins are listed once per `Generator`.
- Sunlight at a point is what neither sphere shadows: night and eclipses.
- The compositor (`render::compose`): the world is drawn once into an HDR
  scene target (`Rgba16Float`, linear light) with its depth kept. A chain of
  full screen stages follows, each reading the colour and depth before it;
  the last, `output`, applies exposure, the chosen tone map (ACES, AgX,
  Khronos neutral, Reinhard, linear) and the target's encoding. No scene
  shader tone maps. An effect is a stage.
- Bloom: what is over a threshold (soft knee) is halved down a five level
  pyramid and summed back up it with a tent filter, then laid over the scene.
  After the clouds, so their silver edges glow too. The threshold is of
  exposed light. Haze is the air's density
  as a factor, in `atmosphere`.
- What glows is what is bright: bloom selects nothing. The moon in the night
  sky is drawn `MOON_SHINE` times a sunlit rock (`moon_shine` in
  `common.wgsl`), over the threshold: not the moon one stands on, nor the
  moon by day (`night_sky`, the rule that drowns the stars). Nothing about
  the picture adapts by itself.
- Clouds are a shell of weather, 1100 to 3000 m over the sea, made of one
  tiling 64^3 noise texture drawn once on the GPU (`render::clouds`). The
  density field (`cloud_field.wgsl`) is in every shader: the compositor
  marches it, and every lit surface asks it for shade along the sun.
- The march runs at half size from the scene depth (two paces: strides in
  clear air, short steps in cloud, both growing with distance), and a full
  size stage lays it over the scene, cut where terrain stands in front.
- At night clouds take the starlight and moonlight the land takes
  (`STARLIGHT`, `moonlight` in `common.wgsl`): pale over dark ground, never
  a hole in it.
- Weather turns about the planet's axis with the clock (the wind), and the
  noise rises through the layer along the local up, so clouds reshape in
  place as they travel. The angle wraps and the rise swings, both on the
  CPU in f64. Cosmetic: not simulated, not stored, the same for a clock.
- Sun shadows: three cascades around the eye (40 m, 400 m, 4 km half side,
  1024 px each), snapped to their texel, in the frame of the nearest body.
  Terrain, volumes, boxes and avatars cast; everything lit by `lit` receives.
- Casters are not the drawn patches: `Frame::shadow_patches` holds built
  leaves before view culling, coarser with distance (1 m, 4 m, 16 m), and
  never schedules generation. Skirts do not cast.
- Cascade count and size live in `render::shadow`, which prepends them to
  every shader; texel sizes ride the view uniform.
- Grass is cosmetic, planet only, on the meadow material alone (forest ground
  is a shade off it and bare), built with the patch from its own samples:
  no generator call. Tier `k` has one tuft per `2^k` half blocks and reaches
  `20 m * 2^k` (six tiers, 640 m), so screen density stays level.
- A tuft is (sector, tier, tier cell): subdivision never moves it. A patch
  carries only the tiers that can reach it before it splits, farthest first;
  the renderer draws the prefix in reach, near patches first, capped.
- `Frame::interaction` is one capsule (the avatar) that bends tufts. Visual
  only. `scene::Effects` turns shadows, grass and clouds off and tunes the clouds
  and exposure (Clients and UI).
- Three separate things hold an avatar. Its **site**: the body it is stored
  relative to, changed at the moon's sphere of influence (4 radii), so it
  rides the orbit. **Gravity**: turns continuously from planet to moon with
  distance; a fifth as strong on the moon. Its own **frame** (`frame_up`):
  turns toward gravity by rotation, fast on foot, in flight only near a
  surface. Flight goes where you look.
- Ground detail is procedural noise anchored to the planet: patch origins are
  wrapped to 1024 m in f64 on the CPU. Rock shows by slope, per pixel.
- The shore (sand, sea floor) shows by height over the sea, per pixel: a
  smooth contour at every LOD. A vertex under it carries the cover of the
  land beside it.
