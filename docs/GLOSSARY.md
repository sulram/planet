# GLOSSARY

Use these terms. Never invent synonyms. A new, renamed or shifted term updates
this file in the same change.

| Term | Meaning |
|---|---|
| Instance | One world as mundos runs it: a container from one version, a world folder and a bucket folder. |
| mundos | The studio's host of worlds (`~/Dev/mundos`): identity, roles, instances, addresses, versions. planet is one of its instance types. |
| Generation | One copy of a world on one version, in mundos. An update makes the next one, and a promotion gives it the world's address. |
| Version | A published image of planet: the core plus the plugins chosen for it, named by its git tag. |
| World folder | Where a world keeps itself: the recipe, what its admin set, and one SQLite file for each plugin that keeps things. |
| Bucket folder | Where a generation's heavy files live in mundos's bucket: a prefix of its own, copied on an update and deleted with the generation. |
| Unfounded | A world as mundos creates it: a door, waiting for its recipe. |
| Founding | The first admin choosing a recipe for an unfounded world, from the offline preview. Done once: it freezes the recipe. |
| Door | mundos's `/enter`: every load passes through it and comes back with a token, or as a guest. |
| Token | What mundos signs for one entry into one world: the account's id, its name and its level, good for a minute. Traded once, on entering, for a key. |
| Key | What the world gives a page that entered with a token: kept in the page's memory, shown on the socket and on a founding. It stands for the life of the page. |
| Level | What a session may do, as mundos says it: `admin`, `builder`, `signed_in`, `anonymous`. |
| Core | What a world is made of and how it behaves, and the host of plugins: today the sphere, a body walking it and the cells. Runs with every plugin off. |
| System | An owner in the core with its words, ops, questions and events: the cells are the first. It knows no tool, key or permission. |
| Plugin | What is done with the core's systems, a tool, a rule or a panel, ours and compiled in, one folder under `plugins/`: a client half and a world half in Rust, a payload on the wire, a panel. On or off for a world. |
| Client half | The part of a plugin the engine hosts: commands in, messages from the server, events out. |
| World half | The part of a plugin the server hosts, in the module: its ops and their levels, what each does, whom an event reaches, what is kept. |
| Module | The one WASM file the server runs through wazero: every plugin's world half, hosted by `world::Host`. Built by `bun run module`, embedded in the Go binary. |
| Bridge | How the server and the module speak: three names, `reserve`, `call` and `host.reply`, and the messages of `proto/planet/module/v1`. |
| Service | What the server offers every plugin and no plugin owns: who a session is, whether it may, the moment, an event told, a store. It carries no feature. |
| Room | What a world half is handed while it applies an op: the moment, who is here, the body's measure, and a way to tell an event to the sessions it picks. |
| Measure | The size of a world's bodies, as its recipe says them: what turns two stances into a distance in blocks. |
| Native plugin | A plugin every version carries: chat, building, land, avatars. |
| Owner | The core or the one plugin that holds a piece of a world's state, and alone changes it. |
| Question | What asks for an answer and changes nothing: a hook or a reading. |
| Hook | A question the core asks its plugins, with a default answer a plugin may answer over: who may do what and where, which avatars are offered. |
| Reading | A question anyone asks an owner about what it holds: the cells in a box, who is near. |
| Event | What an owner says happened, to whoever listens. |
| Host (code) | The part of the core that plugins register with: the registry, the hooks, the stores, the wire's envelope. Distinct from mundos, which hosts worlds. |
| Envelope | How a plugin's message rides the wire: the plugin's name, a kind, and a payload the core never reads. |
| Kind | The name of an op or of an event within its plugin: `say`, `said`. With the plugin's name before it, a command or an event over the seam: `chat.say`. |
| Statement | What a world says it speaks: the engine's version, the wire's, and the plugins that are on. Said at `GET /api/world`, in the welcome, and again when the admin switches a plugin. |
| Layer | What a plugin draws over the world in the web front end, mounted while the plugin is on. |
| Front end | What a person sees and touches over the seam: the Svelte page on the web, egui on the desktop. Sends commands and renders events; the rules are the core's. |
| World | One recipe and one world folder. An instance is exactly one. |
| Recipe | Seed + params + generator version. Enough to regenerate all untouched terrain. |
| Generator | The deterministic function from recipe + address to terrain. Versioned, frozen per world. |
| Source | Where a world's shape comes from: `generated` (plates over the seed) or a field. A recipe param. |
| Field | A cube map of a real body's ground, baked per sector, named by content id and frozen like the generator version. Gives shape, never height. |
| Ruggedness | How far elevation ranges inside one finest texel of a field. What tells a cordillera from a plateau. |
| Plate | One of the tectonic plates a generated world is made of. Its edges are where ranges, trenches and rifts are. |
| Body | A celestial object in a world: the planet, a moon, a micro world. Each has a topology. |
| Topology | The rule set mapping addresses to positions and neighbours. Two kinds: quad sphere, ball. |
| Quad sphere | Six square grids projected on a sphere. Topology of the planet. |
| Grid | Six square faces at one grain. Owns seams, neighbours and the warp; knows no metres. The chunk grid is the block grid coarsened. |
| Sector bits | A body's size: blocks per sector side as a power of two, `4..=16`. A recipe field, frozen per world like the generator version. |
| Reference body | The largest quad sphere, `2^16`. Generator versions are written in its metres; a world's size is the scale they are printed at. |
| Ball | A sphere carved from a plain Cartesian grid. Topology of moons and micro worlds. |
| Sector | One of the six faces of the quad sphere. |
| Seam | The edge where two sectors meet. May swap or flip u and v. |
| Corner | One of 8 points where three sectors meet. Singular; zoned as nature. |
| Shell | A radial layer group with a fixed block count per layer. |
| Build band | The slice of the outer shell where editing is allowed. Bedrock at its floor. |
| Address | Integer location: sector, u, v, h, then chunk and block index. The save format. |
| Address space | The flat grid view where every block is a unit cube. Where simulation runs. |
| Place code | The address as a person says it: a sector digit and up to seven base32 characters, `4-K7M42Q`, plus `@h` in blocks when the ground does not decide the height. A prefix is a box, length is precision. Crockford's alphabet, so it survives being read out. |
| Pose | What a link carries: a body letter, a place code, a bearing and a pitch, `m4-K7M42Q@40,180,-5`. Puts whoever opens it where you stood, looking at what you looked at. |
| Bearing | Degrees clockwise from north, `0..360`. North is the `+Y` pole, the axis the sun turns about. Undefined at a pole. |
| World space | True 3D coordinates. For rendering and flight above the band. |
| Chunk | A 16x16x16 block of cells. Unit of storage, streaming and meshing. |
| Footing | What holds a body up at one direction: the ground under its feet, and any roof over them. Read from the generator, never from a mesh of it. Over nature there is no roof. |
| Terrain layer | Nature: one ground per direction, sampled from the recipe and meshed as quadtree patches. Not editable in world. |
| Build layer | Cubic voxels inside a volume, each air or a paint. The only part of a world with an inside. |
| Paint | What a solid cell is: an index into the palette a volume is shown with. |
| Gesture | Create, delete or paint over a box of cells, with one paint: what a volume applies. What an op carries. |
| Plot | One square of the grid volumes are cut by: 64 by 64 columns of a sector, named by their address less six bits. A volume stands over it. |
| Platform | What a build stands on: a slab of cells one thick, 8 to 64 a side as picked, cut by the address, its top the higher of the highest ground under it and the feet of who asks, on a base. |
| Base | What carries a platform down to the ground, picked as it is laid: a deck, pillars at the corners of every bay and open under the slab; solid, every column filled; floating, nothing. |
| Tool | Create, delete or paint, in hand while building. |
| Stroke | One click and drag of a tool: a slab on the side it started on, or with Alt a wall standing up from it. Lands as one gesture. |
| Ghost | The see-through preview of exactly the cells a stroke would change. |
| Brush | A tool that shapes cells inside a volume: dig, add, smooth, flatten. |
| Stored chunk | A chunk present in the build plugin's store because someone edited it. |
| Generated chunk | A chunk produced on demand from the recipe. Never stored. |
| Copy on first write | The first edit to a chunk generates it, applies the edit and stores it whole. |
| Op | One permission-checked request to an owner to change what it holds. The only way world state changes. |
| Rule | What an op does to what its owner holds: one piece of Rust, compiled into the client and run by the server as WASM. |
| Op log | Append-only record of ops: who, when, address, before, after. |
| Rollback | A volume restored to how it stood at an earlier moment, whoever built in it since. Undo is a person taking back their own strokes. |
| World actor | The single goroutine that owns the world's state and writes. Started by the hub on the first session, gone after the last. |
| Hub | Holds the world's actor. The socket route hands every connection to it. |
| Session | One connection inside a world: a person or agent, from Hello to Left. Numbered by the actor, with one level for its life. |
| Peer | Another session in the same world, as a client sees it: a name, an avatar reference and a stance. |
| Stance | Where a body is and how it moves, as presence carries it fifteen times a second: an address with fractional blocks, a height, a facing, a gait and a speed. A pose is what a link carries; a stance is what a peer sends. |
| Line | One chat message: a scope, a text, the speaker's session and, when they said `@here`, the speaker's place as the actor saw it. Relayed, never stored. |
| Scope | Who hears a line: `near`, within a radius on the same body, or `world`, every session in the world. Never another world. |
| Link (code) | The socket between a client and a world server, as the platform shell holds it. The client owns the protocol, the shell owns the socket. |
| Volume | An integer address box inside one sector, over one plot. The only place voxels exist. |
| Stamp | A flatten and blend footprint, applied when the ground is sampled. How what is not a volume seats into terrain, at every level, without being an edit. The ground it holds is worked earth, where nothing grows. |
| Horizon map | Per texel of a field, the angle of the horizon in two directions. Terrain self shadowing at any range with no shadow map. |
| Land | The native plugin that gives permission to build by volume, to a person's account or an agent's. It answers the permission hook over the level. |
| Landlord | A wish: the role on a volume that builds, subdivides and names builders inside, in the land plugin. |
| Builder | The level `builder`: builds, anywhere in the world. |
| Admin | The level `admin`: founds the world, chooses its plugins, builds. A superadmin of mundos, or an admin of this world there. |
| Visitor | A guest: a session at the level `anonymous`. Walks, under a name of their own choosing. |
| Agent | An AI client without a renderer: an account with the same protocol, levels, commands and readings as a person. |
| Offline preview | The engine with no server: how a planet is looked at before a world is founded, and the desktop explorer. Permanent. |
| Engine | The Rust client as the web app sees it: `shell-web` compiled to WASM. |
| Scene | The plain data a client hands a renderer each frame. Crate `scene`. |
| Patch | One quadtree node of far terrain, meshed as 32x32 quads. |
| Effects | How the picture is made, not what the world is: what a person turns down on a slow machine or tunes to taste. Plain data in every frame. |
| Compositor | The part of the renderer that turns the drawn scene into the picture: a chain of stages ending in `output`. |
| Stage | One full screen pass of the compositor. Reads the colour and depth before it, writes the next target. |
| Tone map | The curve in `output` that turns the scene's unbounded light into what a screen shows. A setting. |
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
| Manifest | `assets/manifest.json`: a version's default asset set. Default avatar, avatars on offer, clip per gait. |
| Gait | How a body moves right now: idle, walk, run, jump, fall, fly. One clip each. |
| Clip | A humanoid animation, retargeted at load so every avatar shares it. |
| Site | The body an avatar's position is stored relative to. Changes at a sphere of influence. |
| Frame | An avatar's own up, view and facing. Turns toward gravity by rotation; in flight it wins. |
| Figure | What an avatar looks like in the client: a worn VRM and its clips, or the box figure. |
| Footprint | What one sample covers: the spacing of the mesh asking the generator, or what a pixel covers of the sea. Detail finer than it is faded out. |
| Anchor | A patch origin wrapped to the detail period in f64: what fixes shader detail to the planet. |
| Entity | A placed thing that is not a voxel: GLB, part, light, field, portal, media frame, script. |
| Asset | A heavy file, named by the hash of its content, in the world's bucket folder. |
| Placement | The use of an asset by an entity. Counts against budgets. |
| Budget | Per-volume limit on triangles, texture size and bytes. |
| Media LOD | Thumbnail far, low version mid, original near. |
| Decoder budget | Max videos decoding at once on a platform. |
| Gravity field | An entity defining "down" inside a shape, with range and priority. |
| Scale band | A discrete avatar scale (giant, human, bug) that selects which levels are streamed. |
| Portal | A link between two (place, scale, world) points. |
| Atlas | The 2D map: the quad sphere unfolded as a cube net. |
| Seam (code) | A single trait where an integration enters the core. Distinct from a sector seam; say "integration seam" when ambiguous. |
| Shell (code) | A platform entry crate: `shell-desktop`, `shell-web`, `shell-xr`. Distinct from a radial shell; say "platform shell" when ambiguous. |
