# Terrain rendering implementation brief

## Objective

- Improve ground-level material detail and the readability of terrain relief,
  while preserving the existing ground-to-orbit quadtree and world recipes.
- Recommended direction: restrained, stylized natural materials with triplanar
  detail, soft material transitions and controlled specular response.
- This is a proposed implementation sequence, not a settled art direction.
- The unresolved terrain look remains in OPEN.md; record the chosen direction
  in DECISIONS.md when implementation settles it.

## Findings from the code

- `crates/client/src/terrain.rs`: 32x32-quad heightfield patches, skirts,
  finite-difference normals and one RGBA color per vertex.
- `crates/render/src/shaders/terrain.wgsl`: no material textures or normal maps.
- Material contract mismatch: `TerrainVertex.color` documents unused alpha,
  `color()` always writes 255, and the shader interprets alpha as gloss.
  Every terrain material therefore receives maximum specular strength.
- `common.wgsl`: directional diffuse light, blue ambient light and a fixed
  specular exponent. Shadow maps have since shipped (ARCHITECTURE, Sea, sky
  and light).
- `Sample::surface_m()` clamps height to sea level. Water shares the terrain
  mesh and cannot currently expose the actual underwater terrain.
- Normals and the steepness color threshold depend on patch sample spacing;
  material appearance can therefore change when LOD changes.
- LOD splits by distance relative to patch width, without projected geometric
  error, hysteresis or a transition morph.
- Patch construction runs synchronously, at four patches per update. This is
  a count budget, not a measured frame-time budget.
- Visibility is checked after subdivision decisions; invisible regions can
  still request finer patches.
- Output clamps directly to the target range. There is no HDR intermediate,
  tone mapping or multisample antialiasing in the inspected terrain path.
- These are implementation findings, not measurements of GPU bottlenecks.

## Visual baseline reviewed

- Rendered ground and orbital PNGs from an isolated copy of commit `f9a5fa9`,
  seed `0000000000000001`, clock `0`, resolution `1280x720`.
- The working tree had an avatar integration in progress; its initial shot
  build failed because a `Frame` initializer lacked `skinned`. No integration
  code was changed for this review.
- The inspected terrain builder, terrain GPU module, terrain/common shaders
  and generator v1 matched that baseline during review.
- Ground: broad uniform sand and grass, conspicuous mountain highlights,
  little shoreline detail, and no avatar cast shadow.
- Orbit: strong atmospheric whitening and coarse material boundaries. These
  captures demonstrate appearance, not streaming stability or performance.

## Delivery 1: correct the material contract

- Capture the baseline before changing shaders, using fixed recipe, camera,
  clock and resolution. Preserve ongoing avatar work in the working tree.
- Make the gloss meaning explicit across `scene`, client and WGSL; reserve
  strong specular response for water and keep dry materials restrained.
- Prefer an explicit material parameter when extending the vertex format for
  delivery 2. Do not continue carrying contradictory alpha semantics.
- Keep lighting computations and material blending in linear color space.
- Check grass, rock, sand, snow and water under the same light.
- Acceptance: dry ground loses the uniform shiny response; water retains a
  distinct highlight; no recipe or generator golden hash changes.

## Delivery 2: material detail, the main visual improvement

- Start with grass, sand and rock. Reuse grass detail with a darker tint for
  Forest, and add a restrained snow material. Forest is currently a material,
  not tree geometry.
- Use small, tileable, legally reusable textures with recorded provenance.
  Include albedo, normal and roughness data; keep a neutral fallback.
- Implement triplanar sampling and normal reorientation in `render`.
  Blend normals in a common frame and normalize the result.
- Keep shader helpers in a dedicated terrain material module; do not turn
  `common.wgsl` into a terrain-only material system.
- Pass material weights or equivalent plain data through `scene`. The
  renderer must not import `worldgen` or derive materials from display RGB.
- Base slope blends on the geometric normal relative to radial up, never
  global Y. Do not use the normal map itself to choose rock versus grass.
- Use smooth weights for rock exposure and biome transitions. Preserve the
  generator's material identity; this is a presentation change.
- Make material masks stable across patch boundaries and LOD. Evaluate any
  slope classification with consistent sampling support, or explicitly blend
  its coarse and fine representations during transitions.
- Anchor detail to the planet, not the camera or each independent patch.
  Compute per-scale wrapped texture origins in CPU f64, then combine them
  with local f32 offsets. Equivalent world points must have equivalent phase
  across patches and sectors; never reconstruct large f32 world positions
  just to sample a small repeating texture.
- Use mipmaps and derivative-aware filtering. Fade fine normal detail with
  distance, retaining the large-scale palette from orbit.
- Bound texture fetches and memory. Measure active layer count and compare a
  lower-cost material path before selecting defaults for weaker devices.
- Keep the same material input usable by future surface nets meshes.
- Acceptance: recognizable ground detail at avatar height; no texture sliding,
  sector seams, stretched cliff textures or obvious material changes at LOD
  boundaries; the orbital view remains coherent.

## Delivery 3: depth and lighting, a separate change

- Add directional shadow mapping near the player, with a bounded distance and
  a lower-cost quality option. Terrain and avatars should cast and receive.
- Fit stable shadow coverage to the visible area and relevant casters. Account
  for casters outside the camera frustum; the current visible patch list alone
  is not sufficient for shadow selection.
- Validate bias and filtering on slopes, along sector seams and at low sun.
- Give ambient light a controlled sky/ground contribution; avoid crushing the
  unlit side of terrain into saturated blue.
- Introduce an HDR target and one final tone-map/output conversion only if
  exposure and highlight clipping still justify the extra pass after review.
- Compare filtered edge quality and MSAA cost before enabling antialiasing by
  default. Keep sample counts consistent across color, depth and pipelines.
- Treat physical atmosphere scattering as its own existing M1 item.

## Delivery 4: interactive grass, after ground materials

- Track this extension in ROADMAP; it is not implemented by this brief.
- Draw simple stylized grass tufts with instancing and vertex-shader bending.
  Vary height, orientation and tint; fix roots while wind bends the tips.
- Let `client` select stable candidates from recipe seed and integer address,
  independent of transient patch IDs and LOD. Filter by ground material and
  slope; exclude water, bare rock and snow. Do not modify generator v1.
- Give boundary candidates one owner to avoid duplicate tufts. Keep roots on
  the rendered surface during LOD transitions without changing candidate IDs.
- Pass plain instance data, simulation time and a bounded set of interaction
  capsules through `scene`; own GPU buffers and WGSL in `render`. Keep this
  cosmetic vegetation separate from persistent entities, physics and edits.
- Use local radial up for growth and the tangent plane for wind and bending.
  Keep positions camera-relative, but anchor wind phase to stable world data.
- Bend away from the avatar capsule with smooth distance falloff, fixed roots
  and bounded displacement. Handle zero lateral distance without NaNs; use
  capsule distance so an avatar flying overhead does not affect the ground.
- Initial interaction is stateless proximity response. Persistent flattened
  trails and timed recovery after departure require state and are out of scope.
- Cap instances and draw distance, reduce density with stable candidate ranks,
  and fade into the ground material. Bound overdraw and include deformation in
  culling bounds; provide reduced-quality and disabled options.
- Use the same deformation for shadow passes if grass casts shadows; measure
  their cost separately. Keep wind reproducible with the fixed scene clock.
- Acceptance: no root sliding, seam duplicates or redistribution on LOD change;
  smooth response when walking or running; no response to distant flight.
  Inspect fixed-clock captures and measure frame time on native and browser.

## Follow-up: terrain stability and responsiveness

- Profile frame time, patch generation, upload cost, draw count and memory on
  native and browser while moving through newly requested terrain.
- Add viewport height to LOD selection and use projected geometric error with
  separate split/merge thresholds. Distance alone does not measure fidelity.
- Derive conservative bounds that include unresolved relief before pruning
  child requests. Current coarse sampled bounds may miss fine peaks.
- Blend child geometry toward the parent's triangulated surface for LOD
  transitions; include normals and material weights in the continuity plan.
- Preserve parent coverage until all required child data is ready. Validate
  neighbouring LOD edges and sector seams; skirts alone do not remove popping.
- Move building behind a bounded job queue with native/browser backends.
  Cancel obsolete recipe jobs and bound uploads as well as generation.
- Keep deterministic `settled_frame()` for screenshots; also exercise actual
  streaming, which settled screenshots cannot validate.

## Boundaries

- Do not change generator v1, topology, collision or saved recipes for this
  visual pass. A future generator change requires a new version.
- Keep the heightfield quadtree. Surface nets remains the planned near-player
  geometry work and is not a prerequisite for better materials.
- A separate water surface with underwater terrain, Fresnel and depth-based
  appearance is a later scoped change, not a shader-only fix to this mesh.
- Keep all wgpu resources in `render`, plain data in `scene`, and terrain
  selection/building in `client`. Check local wgpu 30 source for API details.
- Update ARCHITECTURE and ROADMAP with shipped behavior, DECISIONS with the
  applied choices, and GLOSSARY only if a project term is added or changed.

## Validation and handoff

- Implement deliveries 1 and 2 first; review their captures before expanding.
- Reproducible ground capture:
  `bun run shot --out out/terrain-ground.png --seed 0000000000000001 --clock 0`.
- Reproducible orbital capture:
  `bun run shot --out out/terrain-orbit.png --seed 0000000000000001 --clock 0 --altitude 30000 --pitch -60 --boom 50`.
- Add repeatable coastal, steep-slope, low-sun and sector-seam viewpoints;
  extend shot positioning if the existing altitude/pitch controls cannot reach
  them. Read each before/after PNG rather than only checking file creation.
- Test movement across a patch boundary and a sector seam, rapid flight, a
  recipe change while work is queued, and repeated LOD split/merge cycles.
- Record hardware, resolution, scene and before/after frame times. Do not
  promise a portable FPS target without measurements on the target device.
- Run `bun run check`; verify browser runtime shader validation in addition
  to compiling WASM. Add focused tests for changed data contracts and seam/LOD
  invariants; preserve generator golden tests unchanged.

## Technical references

- [GPU Gems 3, procedural terrain](https://developer.nvidia.com/gpugems/gpugems3/part-i-geometry/chapter-1-generating-complex-procedural-terrains-using-gpu):
  triplanar texturing reference; adapt the technique, not its DX10 pipeline.
- [GPU Gems 2, terrain clipmaps](https://developer.nvidia.com/gpugems/gpugems2/part-i-geometric-complexity/chapter-2-terrain-rendering-using-gpu-based-geometry):
  transition and terrain detail reference; retain this project's quadtree.
