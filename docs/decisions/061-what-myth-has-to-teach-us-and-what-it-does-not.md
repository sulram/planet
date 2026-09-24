# 61. What Myth has to teach us, and what it does not (noted)

Logged 2026-09-22.

`refs/myth` is a Rust rendering engine on wgpu, MIT OR Apache-2.0, about 67,000
lines of Rust and 9,000 of WGSL across ten crates. It is the first reference
inside our stack rather than beside it, which makes its question sharper than
03's: not whether to take an engine, but whether to take this one, written in
our language against our backend. We are not, for three reasons in order of
weight.

**Its scene graph is f32, and a body is planet sized.** `myth_core`'s
`Transform` is a `Vec3`, a `Quat` and an `Affine3A`. There is no `f64` in
`myth_core` or `myth_scene` outside a colour conversion, and nothing anywhere
for a camera-relative frame or an origin shift. Six thousand kilometres from an
origin, one f32 step is about half a metre. Our invariant that no global f32
position exists is written for exactly that, and `render/src/shadow.rs` states
it in its first line: cascade matrices built in f64, acting on camera-relative
f32 positions. Taking Myth means threading f64 through another project's scene
graph and its shadow maths, or dropping the invariant. Neither of those is a
dependency; both are a fork.

**Its floor is a WebGPU device and ours is WebGL2.** Myth passes
`adapter.limits()` as its required limits and takes whatever it finds. Its base
material shaders, `physical.wgsl` and `phong.wgsl`, read `var<storage>`, and
clustered lighting, hi-z, IBL and the splat sort are compute. The string `webgl`
does not occur anywhere in the repository. We ask for
`downlevel_webgl2_defaults()`, `shell-web` builds wgpu with the `webgl` feature
and rejects a browser offering neither, and `skinned.rs` carries its joints in a
uniform block for that reason and says so on its second line. Adopting Myth
deletes the browser fallback and every GLES only device, which is precisely the
ubiquity the targets are chosen for.

**One author, and wgpu 29.** 668 commits between 4 January and 31 July 2026,
every one of them by the same person, at 0.3.0. It moved to wgpu 30 on 8 July
and reverted ten days later. We pin 30 to match vybe so knowledge transfers, so
this is a downgrade now and a wait on one maintainer at every bump after.

**What is worth reading, and may be copied.** MIT OR Apache-2.0 ties Voxelis
(59) as the friendliest licence of anything we hold: unlike Cryptovoxels and
Hyperfy, code and not only ideas could be taken from this one.

The first is the shadow atlas. `crates/myth_render/src/graph/shadow_utils.rs`
packs directional cascades and spot lights into one `Texture2DArray` and point
lights into a `TextureCubeArray`, holds the view projection of every layer in a
single dynamic uniform buffer shared by both, and declares both textures as
transient resources of the render graph. Ours is three cascades of 1024 at 40,
400 and 4000 metres, and the next thing it wants is that packing.

The second is the render graph itself: a pass declares what it reads and
writes, and a compiler in SSA form aliases the memory and inserts the barriers.
`compose.rs` alternates two colour targets by hand and keeps a half size target
and a bloom chain beside them, which is right at today's count of stages and
will not be at three times it. That is the shape to copy when it stops being
right, and copying is what the licence allows.

Clustered forward lighting is filed for the day a voxel world has torches. It
is compute, so it needs the WebGL2 answer Myth never had to write and we would.

**The reframe, since the question was whether we are reinventing a wheel.** Our
renderer is 2,819 lines, and none of this project's hard problems live in it:
determinism across native, WASM and ARM, meshing, cracks between levels,
streaming inside a frame budget. Myth solves none of those and asks 67,000
lines in exchange for 2,819. Graphics that rise with the device is already what
`compose.rs` is for, where an effect is a shader and a line in `Composer::run`,
so tiering by device is a setting rather than an engine.

This is the fourth reference in the family after Dust (56), Veloren (57) and
Voxelis (59), and the first refused on precision rather than on scope.
