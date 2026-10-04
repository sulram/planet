//! The scene: what a client hands to a renderer, as plain data.
//!
//! The client decides *what* is visible; a renderer decides *how* it looks.
//! An agent is a client without a renderer, so nothing here touches the GPU,
//! and the renderer never learns about recipes or addresses.
//!
//! World space is `f64`. Anything that reaches the GPU as `f32` is relative to
//! a nearby `f64` origin (camera-relative rendering, CLAUDE.md invariants).

use bytemuck::{Pod, Zeroable};
use std::ops::Range;
use std::sync::Arc;

use glam::{DAffine3, DQuat, DVec3, Mat4, Vec3};

/// Quads per side of a terrain patch. Every patch shares one index buffer.
pub const PATCH_GRID: u32 = 32;
/// Vertices of the patch surface: a `(PATCH_GRID + 1)^2` grid, row major,
/// `u` along the row.
pub const PATCH_SURFACE_VERTICES: u32 = (PATCH_GRID + 1) * (PATCH_GRID + 1);
/// Surface plus the skirt: one lowered copy of every edge vertex, four runs of
/// `PATCH_GRID + 1` in the order `v = 0`, `v = max`, `u = 0`, `u = max`.
pub const PATCH_VERTICES: u32 = PATCH_SURFACE_VERTICES + 4 * (PATCH_GRID + 1);

/// Stable identity of a terrain patch while it is in the scene.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct PatchId(pub u64);

#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
pub struct TerrainVertex {
    /// Metres from [`TerrainMesh::origin`].
    pub position: [f32; 3],
    pub normal: [f32; 3],
    /// `rgb`: sRGB albedo of the ground cover, before the renderer exposes
    /// rock on slopes. `a`: gloss, 0 for dry ground up to 255 for a mirror.
    pub color: [u8; 4],
}

/// The sea over a patch: the same grid at sea level.
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
pub struct WaterVertex {
    /// Metres from [`TerrainMesh::origin`].
    pub position: [f32; 3],
    /// Water over the ground here, metres. Negative over dry land.
    pub depth_m: f32,
}

/// One terrain patch around an `f64` origin, relative to the centre of its
/// body (see [`PatchDraw`]).
///
/// A patch of the heightfield is [`PATCH_VERTICES`] vertices in the grid
/// [`patch_indices`] describes, and brings no indices of its own. A patch of
/// the volume layer has as many vertices as its surface needs and brings the
/// indices that join them.
#[derive(Clone, Debug)]
pub struct TerrainMesh {
    pub origin: DVec3,
    pub vertices: Vec<TerrainVertex>,
    /// Empty for a heightfield patch, which uses the shared grid.
    pub indices: Vec<u32>,
    /// Present when any of the patch is under the sea. Same layout and index
    /// buffer as `vertices`.
    pub water: Option<Vec<WaterVertex>>,
    /// Cosmetic tufts rooted in this patch, sorted by falling `reach_m`: the
    /// tufts seen from a distance are a prefix. Empty on far patches.
    pub grass: Vec<GrassInstance>,
}

/// How far the densest grass tier is seen, metres. Each sparser tier doubles
/// it. Bounded by the terrain LOD: a patch must split before the tiers it
/// lacks come into reach (`client::grass`).
pub const GRASS_TIER_0_REACH_M: f32 = 20.0;

/// One cosmetic tuft, in the same local space as its terrain patch.
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
pub struct GrassInstance {
    pub root: [f32; 3],
    pub height: f32,
    /// Of the ground at the root. A tuft is lit as the slope it grows on;
    /// it grows along the planet's up, which a renderer knows.
    pub normal: [f32; 3],
    /// The tuft has faded into the ground this far from the eye.
    pub reach_m: f32,
    /// Linear albedo, and rotation about radial up.
    pub color: [f32; 3],
    pub angle: f32,
}

/// The curve that turns the scene's light into what a screen can show.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToneMap {
    /// Filmic, contrasty, saturated: Narkowicz's fit of the ACES curve.
    #[default]
    Aces,
    /// Softer shoulder; bright colours fade to white instead of skewing hue.
    Agx,
    /// Khronos PBR neutral: colours as authored, only highlights compressed.
    Neutral,
    /// Reinhard on luminance: plain and flat.
    Reinhard,
    /// No curve: light above white clips.
    Linear,
}

impl ToneMap {
    /// The order the output shader knows them in.
    pub const ALL: [ToneMap; 5] = [
        ToneMap::Aces,
        ToneMap::Agx,
        ToneMap::Neutral,
        ToneMap::Reinhard,
        ToneMap::Linear,
    ];

    pub fn index(self) -> u32 {
        ToneMap::ALL
            .iter()
            .position(|&each| each == self)
            .expect("listed") as u32
    }
}

/// How the picture is made, not what the world is: what a person may turn
/// down on a slow machine or tune to taste. Travels in every [`Frame`]; a UI
/// sets it through the client's seam. Absent fields take their default.
#[derive(Clone, Copy, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Effects {
    pub shadows: bool,
    pub grass: bool,
    pub clouds: bool,
    /// How much of the sky the weather may fill, 0 to 1.
    pub cloud_cover: f32,
    /// How thick a cloud is to light: 1 is a storm deck, the default a soft
    /// fair weather cloud.
    pub cloud_density: f32,
    /// How fast the weather turns, metres a second at the equator.
    pub wind_m_s: f32,
    /// How fast clouds reshape where they stand, as a factor of the usual.
    pub cloud_change: f32,
    /// What the scene's light is multiplied by before the tone map.
    pub exposure: f32,
    /// How strongly bright light spills over its neighbours. 0 turns it off.
    pub bloom: f32,
    /// How bright light must be to spill, once exposed: 1 is a sunlit wall.
    pub bloom_threshold: f32,
    /// How thick the air is, as a factor of the usual: distance fades sooner.
    pub haze: f32,
    /// How far a swimmer sees under the sea, as a factor of plain sea water.
    pub water_clarity: f32,
    pub tone_map: ToneMap,
}

impl Default for Effects {
    fn default() -> Self {
        Self {
            shadows: true,
            grass: true,
            clouds: true,
            cloud_cover: 0.5,
            cloud_density: 0.1,
            wind_m_s: 14.0,
            cloud_change: 1.0,
            exposure: 1.0,
            bloom: 0.7,
            bloom_threshold: 0.3,
            haze: 0.8,
            water_clarity: 3.0,
            tone_map: ToneMap::Aces,
        }
    }
}

/// Visual interaction only; does not participate in collision.
#[derive(Clone, Copy, Debug)]
pub struct InteractionCapsule {
    pub start: DVec3,
    pub end: DVec3,
    pub radius_m: f32,
}

/// The index buffer shared by every patch. Counter clockwise seen from
/// outside the planet; skirt walls face away from the patch.
pub fn patch_indices() -> Vec<u16> {
    let g = PATCH_GRID;
    let at = |i: u32, j: u32| (j * (g + 1) + i) as u16;
    let mut out = Vec::with_capacity((6 * g * g + 4 * 6 * g) as usize);
    for j in 0..g {
        for i in 0..g {
            out.extend([at(i, j), at(i + 1, j), at(i, j + 1)]);
            out.extend([at(i + 1, j), at(i + 1, j + 1), at(i, j + 1)]);
        }
    }
    // (edge vertex k, skirt run, flipped winding) for the four edges.
    let skirt = |run: u32, k: u32| (PATCH_SURFACE_VERTICES + run * (g + 1) + k) as u16;
    for k in 0..g {
        let edges = [
            (at(k, 0), at(k + 1, 0), 0, false),
            (at(k, g), at(k + 1, g), 1, true),
            (at(0, k), at(0, k + 1), 2, true),
            (at(g, k), at(g, k + 1), 3, false),
        ];
        for (a, b, run, flipped) in edges {
            let (a_low, b_low) = (skirt(run, k), skirt(run, k + 1));
            if flipped {
                out.extend([b, b_low, a_low, b, a_low, a]);
            } else {
                out.extend([a, a_low, b_low, a, b_low, b]);
            }
        }
    }
    out
}

/// A patch to draw, and where its body is this frame. Bodies move (the moon
/// orbits), patches do not: a mesh is built once around its body's centre.
#[derive(Clone, Copy, Debug)]
pub struct PatchDraw {
    pub id: PatchId,
    pub body_center: DVec3,
}

/// The coarsest ground that may cast a shadow, metres across one cell.
///
/// The furthest shadow cascade has a texel of `2 * 4000 / 1024`, about 7.8 m
/// (`render::shadow`). A triangle whose cells are wider than that spans many
/// texels at once, and its depth varies across one of them by far more than
/// any bias can lift, so the whole triangle shadows itself and goes black.
/// Ground coarser than this is lit by the sun's angle alone, which is what a
/// body seen from a distance is lit by anyway.
pub const SHADOW_CASTER_CELL_M: f64 = 4.0;

/// A change to the set of terrain patches a renderer holds.
#[derive(Clone, Debug)]
pub enum TerrainChange {
    Add(PatchId, TerrainMesh),
    Remove(PatchId),
}

/// A unit cube (`-0.5..=0.5`) placed in world space. Avatars are made of these
/// until real models arrive.
#[derive(Clone, Copy, Debug)]
pub struct BoxPart {
    pub transform: DAffine3,
    /// Linear RGB.
    pub color: Vec3,
}

/// Stable identity of a volume mesh while it is in the scene: one chunk of
/// one volume, the ghost of a stroke, or a guide.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct VolumeMeshId(pub u64);

#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
pub struct VolumeVertex {
    /// Metres from [`VolumeMesh::origin`].
    pub position: [f32; 3],
    /// Of the side this vertex is a corner of: a cube is lit flat.
    pub normal: [f32; 3],
    /// `rgb`: sRGB albedo. `a`: how much the side shines with its own
    /// colour, 0 lit by what falls on it up to 255 for a light.
    pub color: [u8; 4],
    /// The corner on its side. First how much of the sky it sees, 0 to 255:
    /// ambient light only. Then where it is across the side and up it, 0 or
    /// 255 each way. Last the edge drawn around the side: 0 none, 1 black,
    /// 2 white.
    pub side: [u8; 4],
}

/// Cubes of the build layer around an `f64` origin in planet space. Every
/// vertex is a corner the client already bent onto the body, so a renderer
/// draws it as it is.
#[derive(Clone, Debug)]
pub struct VolumeMesh {
    pub origin: DVec3,
    pub vertices: Vec<VolumeVertex>,
    pub indices: Vec<u32>,
}

/// A corner of a guide.
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
pub struct GuideVertex {
    /// Metres from [`GuideMesh::origin`].
    pub position: [f32; 3],
    /// Where it is on its side, in cells: a line is drawn wherever either is
    /// whole.
    pub lattice: [f32; 2],
    /// What shows between the lines. `rgb`: sRGB. `a`: how much of it.
    pub color: [u8; 4],
    /// The lines. `rgb`: sRGB. `a`: how much of one shows where cells meet
    /// in eights, and half as much between any two cells.
    pub ink: [u8; 4],
}

/// Sides of cells drawn see-through with a line between each cell and the
/// next: where cells are before any is laid, and what a stroke would change.
/// Bent onto the body by the client, as a [`VolumeMesh`] is.
#[derive(Clone, Debug)]
pub struct GuideMesh {
    pub origin: DVec3,
    pub vertices: Vec<GuideVertex>,
    pub indices: Vec<u32>,
}

/// A light among the cells: the sides that shine in one chunk, taken as one
/// where their middle is. It lights whatever is near, the ground and bodies
/// as well as cells, and casts no shadow.
#[derive(Clone, Copy, Debug)]
pub struct Lamp {
    pub position: DVec3,
    /// Linear RGB: what it gives a side that faces it from a metre away.
    pub color: Vec3,
    /// How far its light goes, metres.
    pub reach_m: f32,
}

/// A change to the set of volume meshes a renderer holds.
#[derive(Clone, Debug)]
pub enum VolumeChange {
    Add(VolumeMeshId, VolumeMesh),
    Guide(VolumeMeshId, GuideMesh),
    Remove(VolumeMeshId),
}

/// Most joints a skinned mesh may have: what fits the smallest uniform buffer
/// every target offers (WebGL2, 16 KiB).
pub const MAX_JOINTS: usize = 128;

/// Stable identity of a skinned mesh while it is in the scene.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct SkinnedMeshId(pub u64);

#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
pub struct SkinnedVertex {
    /// Metres, in the space of the mesh at rest.
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub joints: [u16; 4],
    /// Sum to 1.
    pub weights: [f32; 4],
}

/// An RGBA8 sRGB image, top row first.
#[derive(Clone, Debug)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// A run of indices drawn with one texture.
#[derive(Clone, Debug)]
pub struct SkinnedPrimitive {
    pub indices: Range<u32>,
    /// Index into [`SkinnedMesh::images`]. `None` draws white.
    pub image: Option<usize>,
}

/// A mesh deformed by joints: an avatar.
#[derive(Clone, Debug)]
pub struct SkinnedMesh {
    pub vertices: Vec<SkinnedVertex>,
    pub indices: Vec<u32>,
    pub primitives: Vec<SkinnedPrimitive>,
    pub images: Vec<Image>,
}

/// A change to the set of skinned meshes a renderer holds.
#[derive(Clone, Debug)]
pub enum SkinnedChange {
    Add(SkinnedMeshId, Arc<SkinnedMesh>),
    Remove(SkinnedMeshId),
}

/// One posed skinned mesh in the world.
#[derive(Clone, Debug)]
pub struct SkinnedInstance {
    pub mesh: SkinnedMeshId,
    /// World from mesh space.
    pub transform: DAffine3,
    /// Mesh space from rest space, per joint: at most [`MAX_JOINTS`].
    pub joints: Vec<Mat4>,
}

#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub position: DVec3,
    /// World from camera. The camera looks down its `-Z`, `+Y` is up.
    pub rotation: DQuat,
    /// Vertical field of view, radians.
    pub fov_y: f32,
    /// Near plane, metres. There is no far plane (reversed infinite depth).
    pub near: f32,
}

/// The moon as the light sees it: a sphere that casts a shadow and lends a
/// little light. Its ground is ordinary terrain, in [`Frame::patches`].
#[derive(Clone, Copy, Debug)]
pub struct Moon {
    pub position: DVec3,
    pub radius_m: f64,
}

/// Everything a renderer needs for one frame, besides the patch meshes it
/// already holds.
#[derive(Clone, Debug)]
pub struct Frame {
    pub camera: Camera,
    /// Unit vector toward the sun.
    pub sun_direction: Vec3,
    pub moon: Moon,
    /// Radius of the sea level sphere, metres. The atmosphere sits on it.
    pub planet_radius_m: f64,
    /// World clock, seconds: moves water and wind.
    pub clock_s: f64,
    /// Patches to draw this frame. All were announced by a [`TerrainChange::Add`].
    pub patches: Vec<PatchDraw>,
    /// Loaded terrain selected at shadow LOD, including offscreen casters.
    pub shadow_patches: Vec<PatchDraw>,
    pub interaction: InteractionCapsule,
    pub effects: Effects,
    pub boxes: Vec<BoxPart>,
    pub skinned: Vec<SkinnedInstance>,
    /// The cubes of every volume in reach. All were announced by a
    /// [`VolumeChange::Add`].
    pub volumes: Vec<VolumeMeshId>,
    /// The cubes of glass in reach, drawn over what is solid and blended.
    /// All were announced by a [`VolumeChange::Add`].
    pub glass: Vec<VolumeMeshId>,
    /// The lights among the cells in reach. A view is lit by the nearest.
    pub lamps: Vec<Lamp>,
    /// The stroke a build tool would make: drawn see-through over the world,
    /// cast by nothing. Announced by a [`VolumeChange::Guide`].
    pub ghost: Option<VolumeMeshId>,
    /// The guides to draw over the world. All were announced by a
    /// [`VolumeChange::Guide`].
    pub guides: Vec<VolumeMeshId>,
}
