//! The client core: controller, terrain streaming, and the command/event seam.
//!
//! No window, no DOM, no GPU. A platform shell feeds [`Input`] and a time
//! step, forwards [`Command`]s from its UI, and hands the resulting
//! [`scene::Frame`] and terrain changes to a renderer. An agent drives the
//! same type and simply never renders.

mod avatar;
mod controller;
mod input;
mod seam;
mod terrain;

use glam::DVec3;
use scene::{Frame, TerrainChange};
use topology::{RADIUS_M, SECTOR_SIDE, Sector, SurfacePoint};
pub use worldgen::Recipe;
use worldgen::{GENERATOR_VERSION, Generator, Material};

pub use controller::{Controller, Wish};
pub use input::{Input, Key};
pub use seam::{Command, Event, Mode};
use terrain::Terrain;

/// Seconds for the sun to go around once.
const DAY_S: f64 = 1200.0;
const STATS_EVERY_S: f64 = 0.5;

pub struct Client {
    generator: Generator,
    controller: Controller,
    terrain: Terrain,
    events: Vec<Event>,
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
            terrain: Terrain::default(),
            events: vec![Event::Ready {
                generator_version: GENERATOR_VERSION,
            }],
            aspect: 16.0 / 9.0,
            clock_s: 0.0,
            noon_offset: 0.0,
            stats_timer_s: 0.0,
            frames_since_stats: 0,
            entropy: 0,
        };
        client.face_the_sun();
        client.events.push(Event::RecipeChanged {
            recipe: client.generator.recipe().clone(),
        });
        Ok(client)
    }

    pub fn recipe(&self) -> &Recipe {
        self.generator.recipe()
    }

    pub fn set_aspect(&mut self, aspect: f32) {
        self.aspect = aspect;
    }

    /// Pins the clock, for reproducible headless renders.
    pub fn set_clock(&mut self, seconds: f64) {
        self.clock_s = seconds;
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

    pub fn drain_terrain_changes(&mut self) -> Vec<TerrainChange> {
        self.terrain.drain_changes()
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
        self.controller.update(dt, wish, &self.generator);
        self.stats(dt);

        let camera = self.controller.camera(&self.generator);
        let patches = self.terrain.update(&self.generator, &camera, self.aspect);
        self.frame(camera, patches)
    }

    /// The frame as it would look once streaming caught up. Blocks until every
    /// patch is built: for headless renders only.
    pub fn settled_frame(&mut self) -> Frame {
        let camera = self.controller.camera(&self.generator);
        let patches = self.terrain.settle(&self.generator, &camera, self.aspect);
        self.frame(camera, patches)
    }

    fn frame(&self, camera: scene::Camera, patches: Vec<scene::PatchId>) -> Frame {
        let angle = self.noon_offset + self.clock_s / DAY_S * core::f64::consts::TAU;
        let sun = DVec3::new(angle.cos(), 0.35, angle.sin()).normalize();
        Frame {
            camera,
            sun_direction: sun.as_vec3(),
            planet_radius_m: RADIUS_M,
            patches,
            boxes: avatar::parts(&self.controller),
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
        let dry = !matches!(sample.material, Material::Water | Material::Snow);
        if dry && sample.height_m < 300.0 {
            return point;
        }
        if sample.material != Material::Water {
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
