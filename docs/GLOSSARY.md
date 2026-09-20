# GLOSSARY

Use these terms. Never invent synonyms. A new, renamed or shifted term updates
this file in the same change.

| Term | Meaning |
|---|---|
| World | One recipe + one `world.db` + its records in PocketBase. An instance holds many. |
| Recipe | Seed + params + generator version. Enough to regenerate all untouched terrain. |
| Generator | The deterministic function from recipe + address to terrain. Versioned, frozen per world. |
| Body | A celestial object in a world: the planet, a moon, a micro world. Each has a topology. |
| Topology | The rule set mapping addresses to positions and neighbours. Two kinds: quad sphere, ball. |
| Quad sphere | Six square grids projected on a sphere. Topology of the planet. |
| Ball | A sphere carved from a plain Cartesian grid. Topology of moons and micro worlds. |
| Sector | One of the six faces of the quad sphere. |
| Seam | The edge where two sectors meet. May swap or flip u and v. |
| Corner | One of 8 points where three sectors meet. Singular; zoned as nature. |
| Shell | A radial layer group with a fixed block count per layer. |
| Build band | The slice of the outer shell where editing is allowed. Bedrock at its floor. |
| Address | Integer location: sector, u, v, h, then chunk and block index. The save format. |
| Address space | The flat grid view where every block is a unit cube. Where simulation runs. |
| World space | True 3D coordinates. For rendering and flight above the band. |
| Chunk | A 16x16x16 block of cells. Unit of storage, streaming and meshing. |
| Terrain layer | Smooth voxels: density + material, meshed by surface nets. |
| Build layer | Cubic voxels: block types, plus ramp, wedge and half slab shapes. |
| Brush | A terrain edit tool: dig, add, smooth, flatten, paint. |
| Stored chunk | A chunk present in `world.db` because someone edited it. |
| Generated chunk | A chunk produced on demand from the recipe. Never stored. |
| Copy on first write | The first edit to a chunk generates it, applies the edit and stores it whole. |
| Op | One permission-checked edit request. The only way world state changes. |
| Op log | Append-only record of ops: who, when, address, before, after. |
| Hot plane | Chunks, ops, presence, streaming. Our code, `world.db`, WebSocket. |
| Cold plane | Accounts, worlds, volumes, roles, records. PocketBase. |
| World actor | The single goroutine that owns one active world's state and writes. |
| Volume | An integer address box inside one sector where building is granted. Nests. |
| Landlord | Role on a volume: build, subdivide, grant roles inside. |
| Builder | Role on a volume: build only. |
| Admin | Role on a world: build anywhere. Still logged. |
| Operator | Global user flag (`users.operator`). Runs the instance, may use the backoffice. Not the superuser, not a world admin. |
| Superuser | The PocketBase panel login. Infrastructure, never a person in a world. |
| Backoffice | The operator pages of the web app: worlds and users. |
| Magic link | The sign in email: a one click link plus the same code to type on native. |
| Visitor | Anyone without a role here, including anonymous. Looks, never builds. |
| Agent | An AI client without a renderer, authenticated by an API token. |
| Offline preview | The engine with no server: `/play` and the desktop explorer. Permanent. |
| Engine | The Rust client as the web app sees it: `shell-web` compiled to WASM. |
| Scene | The plain data a client hands a renderer each frame. Crate `scene`. |
| Patch | One quadtree node of far terrain, meshed as 32x32 quads. |
| Compositor | The part of the renderer that turns the drawn scene into the picture: a chain of stages ending in `output`. |
| Stage | One full screen pass of the compositor. Reads the colour and depth before it, writes the next target. |
| Scene target | The HDR, linear light texture the world is drawn into, with its depth. |
| Weather | The low frequency field that says where on the planet clouds may stand at all. Turns with the clock. |
| Cloud layer | The shell of altitude clouds live in. Marched by the compositor, sampled by lit surfaces for shade. |
| Cascade | One sun shadow map of a nested set around the eye; each covers more ground at a coarser texel. |
| Caster | A patch, box or avatar drawn into a cascade. Chosen apart from what the camera sees. |
| Tuft | One cosmetic grass instance, identified by (sector, tier, tier cell). Not an entity, not stored. |
| Grass tier | A density level of tufts: tier `k` spaces them `2^k` half blocks apart and reaches twice as far as tier `k-1`. |
| Interaction capsule | A capsule the renderer bends grass away from. Visual only, no collision. |
| Tangent warp | The quad sphere mapping: `tan(s * pi / 4)` on the cube face. |
| Avatar | The VRM body a person wears. Named by an asset reference, never by index. |
| Asset reference | A path under the asset root, or an absolute URL. How the client names any file it wants. |
| Manifest | `assets/manifest.json`: the instance's default set. Default avatar, avatars on offer, clip per gait. |
| Gait | How a body moves right now: idle, walk, run, jump, fall, fly. One clip each. |
| Clip | A humanoid animation, retargeted at load so every avatar shares it. |
| Site | The body an avatar's position is stored relative to. Changes at a sphere of influence. |
| Frame | An avatar's own up, view and facing. Turns toward gravity by rotation; in flight it wins. |
| Figure | What an avatar looks like in the client: a worn VRM and its clips, or the box figure. |
| Footprint | The spacing of the mesh asking the generator for terrain. Detail finer than it is faded out. |
| Anchor | A patch origin wrapped to the detail period in f64: what fixes shader detail to the planet. |
| Entity | A placed thing that is not a voxel: GLB, part, light, field, portal, media frame, script. |
| Asset | A file, global, named by its content hash. |
| Placement | The use of an asset by an entity in a world. Counts against budgets. |
| Budget | Per-volume limit on triangles, texture size and bytes. |
| Quota | Per-user storage allowance in bytes. |
| Media LOD | Thumbnail far, low version mid, original near. |
| Decoder budget | Max videos decoding at once on a platform. |
| Gravity field | An entity defining "down" inside a shape, with range and priority. |
| Scale band | A discrete avatar scale (giant, human, bug) that selects which levels are streamed. |
| Portal | A link between two (place, scale, world) points. |
| Atlas | The 2D map: the quad sphere unfolded as a cube net. |
| Seam (code) | A single trait where an integration enters the core. Distinct from a sector seam; say "integration seam" when ambiguous. |
| Shell (code) | A platform entry crate: `shell-desktop`, `shell-web`, `shell-xr`. Distinct from a radial shell; say "platform shell" when ambiguous. |
