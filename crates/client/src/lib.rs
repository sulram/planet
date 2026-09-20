//! The client core: controller, terrain streaming, and the command/event seam.
//!
//! No window, no DOM, no GPU. A platform shell feeds [`Input`] and a time
//! step, forwards [`Command`]s from its UI, and hands the resulting
//! [`scene::Frame`] and terrain changes to a renderer. An agent drives the
//! same type and simply never renders.

mod assets;
mod box_figure;
mod controller;
mod figure;
mod grass;
mod input;
mod seam;
mod terrain;

use glam::DVec3;
use scene::{SkinnedChange, TerrainChange};
use topology::{RADIUS_M, SECTOR_SIDE, Sector, SurfacePoint};
pub use worldgen::Recipe;
use worldgen::{GENERATOR_VERSION, Generator, Material};

pub use assets::AssetRequest;
use assets::{MANIFEST_PATH, Manifest, Purpose, Requests};
pub use controller::{Controller, Wish};
use figure::Figure;
pub use input::{Input, Key};
pub use scene::{Effects, Frame};
pub use seam::{Command, Event, Mode};
use terrain::{Body, Terrain};

/// Seconds for the sun to go around once.
const DAY_S: f64 = 1200.0;
/// Turns of the moon per turn of the sun.
const MOON_PACE: f64 = 0.93;
/// Centre of the planet to centre of the moon, metres.
const MOON_ORBIT_M: f64 = 160_000.0;
use worldgen::MOON_RADIUS_M;
const STATS_EVERY_S: f64 = 0.5;

pub struct Client {
    generator: Generator,
    controller: Controller,
    terrain: Terrain,
    moon_terrain: Terrain,
    figure: Figure,
    requests: Requests,
    manifest: Option<Manifest>,
    /// A random avatar was asked for before the manifest arrived.
    wants_random_avatar: bool,
    /// The asset reference of the avatar asked for last.
    wanted_avatar: Option<String>,
    events: Vec<Event>,
    effects: Effects,
    /// Width over height of the view, for culling.
    aspect: f32,
    clock_s: f64,
    /// Sun angle at `clock_s = 0`, chosen so every spawn starts in daylight.
    noon_offset: f64,
    stats_timer_s: f64,
    frames_since_stats: u32,
    entropy: u64,
}

impl Client {
    pub fn new(recipe: Recipe) -> Result<Client, worldgen::RecipeError> {
        let generator = Generator::new(recipe)?;
        let controller = Controller::spawn(spawn_point(&generator), &generator);
        let mut client = Client {
            generator,
            controller,
            terrain: Terrain::new(Body::Planet),
            moon_terrain: Terrain::new(Body::Moon),
            figure: Figure::default(),
            requests: Requests::default(),
            manifest: None,
            wants_random_avatar: false,
            wanted_avatar: None,
            events: vec![Event::Ready {
                generator_version: GENERATOR_VERSION,
            }],
            effects: Effects::default(),
            aspect: 16.0 / 9.0,
            clock_s: 0.0,
            noon_offset: 0.0,
            stats_timer_s: 0.0,
            frames_since_stats: 0,
            entropy: 0,
        };
        client
            .requests
            .ask(MANIFEST_PATH.to_owned(), Purpose::Manifest);
        client.face_the_sun();
        client.events.push(Event::RecipeChanged {
            recipe: client.generator.recipe().clone(),
        });
        client.events.push(Event::EffectsChanged {
            effects: client.effects,
        });
        Ok(client)
    }

    /// Takes what a UI asked for, within what is sane, and says what it took.
    fn set_effects(&mut self, effects: Effects) {
        let sane = |value: f32, low: f32, high: f32, fallback: f32| {
            if value.is_finite() {
                value.clamp(low, high)
            } else {
                fallback
            }
        };
        let usual = Effects::default();
        self.effects = Effects {
            cloud_cover: sane(effects.cloud_cover, 0.0, 1.0, usual.cloud_cover),
            cloud_density: sane(effects.cloud_density, 0.1, 3.0, usual.cloud_density),
            wind_m_s: sane(effects.wind_m_s, 0.0, 80.0, usual.wind_m_s),
            cloud_change: sane(effects.cloud_change, 0.0, 6.0, usual.cloud_change),
            exposure: sane(effects.exposure, 0.1, 4.0, usual.exposure),
            ..effects
        };
        self.events.push(Event::EffectsChanged {
            effects: self.effects,
        });
    }

    pub fn recipe(&self) -> &Recipe {
        self.generator.recipe()
    }

    pub fn set_aspect(&mut self, aspect: f32) {
        self.aspect = aspect;
    }

    /// Stirs in randomness only a platform has (a clock, the browser). Without
    /// it every random choice is reproducible, which headless shots rely on.
    pub fn add_entropy(&mut self, bits: u64) {
        self.entropy ^= mix(bits);
    }

    /// Pins the clock, for reproducible headless renders.
    pub fn set_clock(&mut self, seconds: f64) {
        self.clock_s = seconds;
    }

    /// Moves the avatar to a place in sector 0, `u` and `v` in `0..=1`. For
    /// previews; travel in a world is walking, flying and portals.
    pub fn teleport(&mut self, u: f64, v: f64) {
        let side = f64::from(SECTOR_SIDE);
        let point = SurfacePoint::new(Sector::ALL[0], u * side, v * side);
        self.controller.teleport(point, &self.generator);
    }

    /// Flies to `gap_m` metres under the moon, on the side that faces the
    /// planet, and aims the camera. For previews.
    pub fn visit_moon(&mut self, gap_m: f64, pitch: f64, boom_m: f64) {
        self.controller.pose_view(pitch, boom_m);
        let moon = self.moon_position();
        let below = moon.normalize() * (moon.length() - MOON_RADIUS_M - gap_m);
        self.controller.set_moon(controller::MoonBody {
            center: moon,
            radius_m: MOON_RADIUS_M,
        });
        self.controller.fly_to(below);
    }

    /// Poses the avatar and camera for a preview: see [`Controller::pose`].
    pub fn pose(&mut self, altitude_m: f64, pitch: f64, boom_m: f64) {
        self.controller
            .pose(altitude_m, pitch, boom_m, &self.generator);
    }

    pub fn command(&mut self, command: Command) {
        match command {
            Command::SetRecipe { recipe } => match Generator::new(recipe) {
                Ok(generator) => self.regenerate(generator),
                Err(error) => self.events.push(Event::Rejected {
                    message: error.to_string(),
                }),
            },
            Command::SetMode { mode } => self.set_mode(mode),
            Command::SetAvatar { path } => self.wear(path),
            Command::RandomAvatar => self.random_avatar(),
            Command::NextAvatar => self.next_avatar(),
            Command::SetEffects { effects } => self.set_effects(effects),
        }
    }

    /// Commands arriving as JSON over the seam. Bad JSON becomes a
    /// [`Event::Rejected`], never a panic: the other side is a UI.
    pub fn command_json(&mut self, json: &str) {
        match Command::from_json(json) {
            Ok(command) => self.command(command),
            Err(error) => self.events.push(Event::Rejected {
                message: error.to_string(),
            }),
        }
    }

    pub fn drain_events(&mut self) -> Vec<Event> {
        core::mem::take(&mut self.events)
    }

    /// Files the client wants. The shell fetches each one and answers with
    /// [`Client::asset_loaded`].
    pub fn drain_asset_requests(&mut self) -> Vec<AssetRequest> {
        self.requests.drain()
    }

    /// The answer to an [`AssetRequest`]: the bytes, or why there are none.
    /// A bad or missing file is logged as a rejection; the box figure and the
    /// rest pose are the fallbacks, so nothing breaks.
    pub fn asset_loaded(&mut self, id: u64, bytes: Result<Vec<u8>, String>) {
        let Some(purpose) = self.requests.answer(id) else {
            return;
        };
        let outcome = bytes.and_then(|bytes| match purpose {
            Purpose::Manifest => serde_json::from_slice::<Manifest>(&bytes)
                .map(|manifest| self.adopt(manifest))
                .map_err(|e| e.to_string()),
            Purpose::Avatar => avatar::Avatar::from_vrm(&bytes)
                .map(|avatar| self.worn(avatar))
                .map_err(|e| e.to_string()),
            Purpose::Clip(gait) => avatar::Clip::from_glb(&bytes)
                .map(|clip| self.figure.add_clip(gait, clip))
                .map_err(|e| e.to_string()),
        });
        if let Err(message) = outcome {
            self.events.push(Event::Rejected {
                message: format!("{purpose:?}: {message}"),
            });
            if purpose == Purpose::Avatar {
                self.wear_default();
            }
        }
    }

    /// Asks for the avatar at an asset reference. Every way of choosing an
    /// avatar ends here, whether the reference came from the manifest, a
    /// cookie or, later, a user's own uploads.
    fn wear(&mut self, path: String) {
        self.requests.ask(path.clone(), Purpose::Avatar);
        self.wanted_avatar = Some(path);
    }

    fn worn(&mut self, avatar: avatar::Avatar) {
        self.figure.wear(avatar);
        if let Some(path) = self.wanted_avatar.clone() {
            self.events.push(Event::AvatarChanged { path });
        }
    }

    /// The wanted avatar could not be loaded: there is always a default.
    fn wear_default(&mut self) {
        let default = self
            .manifest
            .as_ref()
            .and_then(|m| m.default_avatar.clone());
        match default {
            Some(path) if self.wanted_avatar.as_ref() != Some(&path) => self.wear(path),
            // The default itself failed, or there is none: the box figure stays.
            _ => self.wanted_avatar = None,
        }
    }

    /// The avatar on offer after the one worn, wrapping around.
    fn next_avatar(&mut self) {
        let Some(manifest) = &self.manifest else {
            return;
        };
        if manifest.avatars.is_empty() {
            return;
        }
        let current = self.wanted_avatar.as_ref();
        let index = manifest
            .avatars
            .iter()
            .position(|path| Some(path) == current);
        let next = index.map_or(0, |i| (i + 1) % manifest.avatars.len());
        self.wear(manifest.avatars[next].clone());
    }

    /// The manifest arrived: fetch the clips it names, and the avatar if one
    /// was waiting for it.
    fn adopt(&mut self, manifest: Manifest) {
        for (gait, path) in &manifest.clips {
            self.requests.ask(path.clone(), Purpose::Clip(*gait));
        }
        self.manifest = Some(manifest);
        if core::mem::take(&mut self.wants_random_avatar) {
            self.random_avatar();
        }
    }

    fn random_avatar(&mut self) {
        let Some(manifest) = &self.manifest else {
            self.wants_random_avatar = true;
            return;
        };
        if manifest.avatars.is_empty() {
            return;
        }
        let pick = mix(self.entropy) as usize % manifest.avatars.len();
        self.wear(manifest.avatars[pick].clone());
    }

    pub fn drain_skinned_changes(&mut self) -> Vec<SkinnedChange> {
        self.figure.drain_changes()
    }

    pub fn drain_terrain_changes(&mut self) -> Vec<TerrainChange> {
        let mut changes = self.terrain.drain_changes();
        changes.extend(self.moon_terrain.drain_changes());
        changes
    }

    /// Advances the simulation by `dt` seconds and returns the frame to draw.
    pub fn update(&mut self, dt: f64, input: &mut Input) -> Frame {
        let (pressed, look, zoom) = input.take_frame();
        self.entropy = self.entropy.wrapping_add(dt.to_bits()).rotate_left(7);
        for key in pressed {
            match key {
                Key::ToggleMode => self.set_mode(match self.controller.mode {
                    Mode::Walk => Mode::Fly,
                    Mode::Fly => Mode::Walk,
                }),
                Key::NewSeed => {
                    let recipe = Recipe::new(mix(self.recipe().seed ^ self.entropy));
                    self.command(Command::SetRecipe { recipe });
                }
                Key::NextAvatar => self.next_avatar(),
                _ => {}
            }
        }

        // Long frames (a stalled tab) must not tunnel the avatar anywhere.
        let dt = dt.clamp(0.0, 0.1);
        self.clock_s += dt;
        let wish = Wish {
            movement: input.movement(),
            up: input.held(Key::Up),
            down: input.held(Key::Down),
            sprint: input.held(Key::Sprint),
            look,
            zoom,
        };
        // The moon has a gravity field of its own: fly into it and it becomes
        // down; turn flight off there and you fall to it, walk and jump high.
        let moon = self.moon_position();
        self.controller.set_moon(controller::MoonBody {
            center: moon,
            radius_m: MOON_RADIUS_M,
        });
        let streamer = if self.controller.on_moon() {
            &self.moon_terrain
        } else {
            &self.terrain
        };
        let footprint_m = streamer.drawn_footprint_m(self.controller.radial());
        self.controller.set_drawn_footprint(footprint_m);
        self.controller.update(dt, wish, &self.generator);
        self.figure.update(dt, &self.controller);
        self.stats(dt);

        let camera = self.controller.camera(&self.generator);
        let patches = self.stream(&camera, Terrain::update);
        self.frame(camera, patches)
    }

    /// The frame as it would look once streaming caught up. Blocks until every
    /// patch is built: for headless renders only.
    pub fn settled_frame(&mut self) -> Frame {
        let camera = self.controller.camera(&self.generator);
        let patches = self.stream(&camera, Terrain::settle);
        self.frame(camera, patches)
    }

    /// Where the moon is now: an orbit on rails, a function of the clock. It
    /// laps the sky a little slower than the sun on a tilted path, so its
    /// phase changes from night to night.
    fn moon_position(&self) -> DVec3 {
        let angle = self.noon_offset + self.clock_s / DAY_S * core::f64::consts::TAU;
        let lunar = angle * MOON_PACE + 2.4;
        let tilt = -0.25 + 0.3 * (lunar * 0.37).sin();
        DVec3::new(lunar.cos(), tilt, lunar.sin()).normalize() * MOON_ORBIT_M
    }

    /// Streams the terrain of every body and lists what to draw. Each
    /// streamer works around its own body's centre, so the moon's sees the
    /// camera from where the moon is now.
    fn stream(
        &mut self,
        camera: &scene::Camera,
        select: fn(&mut Terrain, &Generator, &scene::Camera, f32) -> Vec<scene::PatchId>,
    ) -> Vec<scene::PatchDraw> {
        let moon = self.moon_position();
        let from_moon = scene::Camera {
            position: camera.position - moon,
            ..*camera
        };
        let on_planet = select(&mut self.terrain, &self.generator, camera, self.aspect);
        let on_moon = select(
            &mut self.moon_terrain,
            &self.generator,
            &from_moon,
            self.aspect,
        );
        let at = |body_center: DVec3| move |id| scene::PatchDraw { id, body_center };
        let planet = on_planet.into_iter().map(at(DVec3::ZERO));
        planet.chain(on_moon.into_iter().map(at(moon))).collect()
    }

    fn frame(&self, camera: scene::Camera, patches: Vec<scene::PatchDraw>) -> Frame {
        let angle = self.noon_offset + self.clock_s / DAY_S * core::f64::consts::TAU;
        let sun = DVec3::new(angle.cos(), 0.35, angle.sin()).normalize();
        Frame {
            camera,
            sun_direction: sun.as_vec3(),
            moon: scene::Moon {
                position: self.moon_position(),
                radius_m: MOON_RADIUS_M,
            },
            planet_radius_m: RADIUS_M,
            clock_s: self.clock_s,
            patches,
            shadow_patches: self
                .terrain
                .shadow_patches()
                .iter()
                .map(|&id| scene::PatchDraw {
                    id,
                    body_center: DVec3::ZERO,
                })
                .chain(
                    self.moon_terrain
                        .shadow_patches()
                        .iter()
                        .map(|&id| scene::PatchDraw {
                            id,
                            body_center: self.moon_position(),
                        }),
                )
                .collect(),
            effects: self.effects,
            interaction: scene::InteractionCapsule {
                start: self.controller.position() + self.controller.up() * 0.2,
                end: self.controller.position() + self.controller.up() * 1.5,
                radius_m: 0.65,
            },
            boxes: if self.figure.is_worn() {
                Vec::new()
            } else {
                box_figure::parts(&self.controller)
            },
            skinned: self.figure.instance(&self.controller).into_iter().collect(),
        }
    }

    fn set_mode(&mut self, mode: Mode) {
        if self.controller.mode != mode {
            self.controller.mode = mode;
            self.events.push(Event::ModeChanged { mode });
        }
    }

    fn regenerate(&mut self, generator: Generator) {
        let mode = self.controller.mode;
        self.generator = generator;
        self.controller = Controller::spawn(spawn_point(&self.generator), &self.generator);
        self.controller.mode = mode;
        self.terrain.clear();
        self.moon_terrain.clear();
        self.face_the_sun();
        self.events.push(Event::RecipeChanged {
            recipe: self.generator.recipe().clone(),
        });
    }

    /// Mid morning at the spawn, whatever the clock says.
    fn face_the_sun(&mut self) {
        let up = self.controller.up();
        let now = self.clock_s / DAY_S * core::f64::consts::TAU;
        self.noon_offset = up.z.atan2(up.x) - 0.7 - now;
    }

    fn stats(&mut self, dt: f64) {
        self.stats_timer_s += dt;
        self.frames_since_stats += 1;
        if self.stats_timer_s >= STATS_EVERY_S {
            self.events.push(Event::Stats {
                fps: (f64::from(self.frames_since_stats) / self.stats_timer_s) as f32,
                altitude_m: self.controller.altitude_m(&self.generator),
                speed_mps: self.controller.speed_mps(),
                sector: self.controller.point().sector.index() as u8,
            });
            self.stats_timer_s = 0.0;
            self.frames_since_stats = 0;
        }
    }
}

/// A place to stand: the first gentle, dry ground along a fixed search path
/// over sector 0. Deterministic per recipe, so a world always opens the same.
fn spawn_point(generator: &Generator) -> SurfacePoint {
    let side = f64::from(SECTOR_SIDE);
    let sector = Sector::ALL[0];
    // A coarse lattice ordered from the sector centre outward.
    let mut cells: Vec<(i32, i32)> = (-12..=12)
        .flat_map(|i| (-12..=12).map(move |j| (i, j)))
        .collect();
    cells.sort_by_key(|(i, j)| i * i + j * j);
    let candidates = cells.into_iter().map(|(i, j)| {
        SurfacePoint::new(
            sector,
            side * (0.5 + f64::from(i) / 26.0),
            side * (0.5 + f64::from(j) / 26.0),
        )
    });
    let mut fallback = None;
    for point in candidates {
        let sample = generator.sample(point.direction());
        let dry = sample.height_m > 1.0 && sample.material != Material::Snow;
        if dry && sample.height_m < 300.0 {
            return point;
        }
        if sample.height_m > 0.0 {
            fallback.get_or_insert(point);
        }
    }
    // A water world, or one frozen solid: stand on whatever there is.
    fallback.unwrap_or(SurfacePoint::new(sector, side / 2.0, side / 2.0))
}

/// splitmix64 finalizer: spreads a counter into a seed.
fn mix(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}
