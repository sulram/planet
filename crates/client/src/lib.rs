//! The client core: controller, terrain streaming, and the command/event seam.
//!
//! No window, no DOM, no GPU. A platform shell feeds [`Input`] and a time
//! step, forwards [`Command`]s from its UI, hands over the frames of a link to
//! a world server and sends back what is queued for it, and hands the
//! resulting [`scene::Frame`], terrain and volume changes to a renderer. An agent
//! drives the same type and simply never renders.

mod assets;
mod box_figure;
mod cells;
pub mod collision;
mod controller;
mod figure;
mod grass;
mod history;
mod input;
mod parts;
mod peers;
mod pictures;
mod place;
mod plugin;
mod seam;
mod session;
mod terrain;
mod wardrobe;

use glam::{DAffine3, DMat3, DQuat, DVec3};
use scene::{ModelChange, PictureChange, SkinnedChange, TerrainChange, VolumeChange};
use topology::{Sector, SurfacePoint};
pub use worldgen::{Field, Recipe};
use worldgen::{GENERATOR_VERSION, Generator, Material, Params, Sample};

pub use assets::{AssetRequest, Form};
use assets::{MANIFEST_PATH, Manifest, Purpose, Requests};
pub use cells::{Aim, Cells, Guide, Met, PALETTE, REACH_M, Refusal, Seat, Sight};
pub use collision::BODY_M;
pub use controller::{Built, Controller, Wish};
use figure::{Clips, Figure, Gait, Motion};
pub use history::{History, Owner};
pub use input::{Chord, Input, Key, KeyAsk};
pub use part::{
    ContentId, ImageKind, Media, NAME_BYTES, Name, Part, PartId, PictureLevel, Portal,
    Pose as PartPose, Shape,
};
pub use parts::is_lamp;
pub use parts::{CUBE, Landmarked, Mark, PartRefusal, Parts, VOXEL};
use peers::Peers;
use pictures::Pictures;
pub use pictures::{FULL_SIDE, LOW_SIDE, Screen, THUMB_SIDE};
pub use place::Pose;
pub use plugin::{Eye, Feet, Host, PartHit, Plugin, Standing, Turn};
use plugin::{Lent, Plugins};
pub use scene::{Effects, Frame, ToneMap};
pub use seam::{Anchor, Command, Event, Label, Level, Mode, PeerInfo, PluginOn, SessionStatus};
pub use session::Outbound;
use session::Session;
use terrain::{Body, Cover, Terrain, Viewpoint};
use wardrobe::Wardrobe;

/// Seconds for the sun to go around once.
const DAY_S: f64 = 1200.0;
/// Turns of the moon per turn of the sun.
const MOON_PACE: f64 = 0.93;
/// Centre of the planet to centre of the moon, metres: twenty of its radii,
/// so it fills as much of the sky whatever its size.
const MOON_ORBIT_M: f64 = 20.0 * MOON_RADIUS_M;
use worldgen::MOON_RADIUS_M;
const STATS_EVERY_S: f64 = 0.5;

pub struct Client {
    generator: Generator,
    /// Kept so a new recipe over the same ground needs no second download.
    field: Option<Field>,
    controller: Controller,
    terrain: Terrain,
    moon_terrain: Terrain,
    /// The cells of the world: its volumes, and what is drawn of them.
    cells: Cells,
    /// The parts of the world: what is placed that is no cell.
    parts: Parts,
    /// Every change a hand made here, in order, of whatever owner, to take
    /// back and put back as one list (DECISIONS 115).
    history: History,
    /// What the last world to welcome this client said it may do. `None`
    /// until one has: with no world, nothing is changed (DECISIONS 104).
    level: Option<Level>,
    /// The plugin that has the pointer, while one does: the pointer stays
    /// free to aim and its button is that plugin's.
    pointer: Option<&'static str>,
    /// The player's own body.
    figure: Figure,
    /// The clips every figure shares.
    clips: Clips,
    /// Every avatar loaded, by asset reference, worn by any number of bodies.
    wardrobe: Wardrobe,
    pictures: Pictures,
    session: Session,
    peers: Peers,
    /// What a shell plugged in, and which of it the world has on.
    plugins: Plugins,
    requests: Requests,
    manifest: Option<Manifest>,
    /// A random avatar was asked for before the manifest arrived.
    wants_random_avatar: bool,
    /// The asset reference of the avatar asked for last.
    wanted_avatar: Option<String>,
    /// What to be called, as the front end set it. Rides in Hello.
    wanted_name: String,
    events: Vec<Event>,
    effects: Effects,
    /// Width over height of the view, for culling.
    aspect: f32,
    /// The view's height in pixels: what a picture's size on the screen is
    /// measured in.
    view_height_px: f32,
    /// The world's clock, seconds. Online it is the server's, set by Welcome,
    /// so every client shares one sky; offline it counts from the world's
    /// making. The sun, the moon and the weather are functions of it.
    clock_s: f64,
    /// Sun angle at `clock_s = 0`: mid morning at the spawn, a fact of the
    /// world, so clients that set their clocks by one server agree on the sun.
    noon_offset: f64,
    stats_timer_s: f64,
    frames_since_stats: u32,
    /// Whether the last frame hung anchors, so the first empty frame after
    /// them still says so.
    had_anchors: bool,
    had_labels: bool,
    /// A body's passage through a portal, while it lasts.
    passage: Option<Passage>,
    /// Whether the last frame's streaming found nothing to build.
    settled: bool,
    entropy: u64,
}

impl Client {
    /// A world that needs only its seed. A recipe that names a field is
    /// refused here: that one needs [`Client::with_field`].
    pub fn new(recipe: Recipe) -> Result<Client, worldgen::RecipeError> {
        Client::build(Generator::new(recipe)?, None)
    }

    /// A world shaped by a baked field. The field is kept, so a later
    /// `SetRecipe` onto the same ground costs nothing but the respawn.
    pub fn with_field(recipe: Recipe, field: Field) -> Result<Client, worldgen::RecipeError> {
        let generator = Generator::with_field(recipe, field.clone())?;
        Client::build(generator, Some(field))
    }

    fn build(generator: Generator, field: Option<Field>) -> Result<Client, worldgen::RecipeError> {
        let controller = Controller::spawn(spawn_point(&generator), &generator);
        // The body every streamer is printed at. Frozen with the recipe (49).
        let sphere = generator.sphere();
        let mut client = Client {
            generator,
            field,
            controller,
            terrain: Terrain::new(Body::new(terrain::Kind::Planet, sphere)),
            moon_terrain: Terrain::new(Body::new(terrain::Kind::Moon, sphere)),
            cells: Cells::default(),
            parts: Parts::default(),
            history: History::default(),
            level: None,
            pointer: None,
            figure: Figure::default(),
            clips: Clips::new(),
            wardrobe: Wardrobe::default(),
            pictures: Pictures::default(),
            session: Session::default(),
            peers: Peers::default(),
            plugins: Plugins::default(),
            requests: Requests::default(),
            manifest: None,
            wants_random_avatar: false,
            wanted_avatar: None,
            wanted_name: String::new(),
            events: vec![
                Event::Ready {
                    generator_version: GENERATOR_VERSION,
                },
                Event::Palette {
                    colors: PALETTE
                        .iter()
                        .map(|[r, g, b]| format!("#{r:02x}{g:02x}{b:02x}"))
                        .collect(),
                },
            ],
            effects: Effects::default(),
            aspect: 16.0 / 9.0,
            view_height_px: 1080.0,
            clock_s: 0.0,
            noon_offset: 0.0,
            stats_timer_s: 0.0,
            frames_since_stats: 0,
            had_anchors: false,
            had_labels: false,
            passage: None,
            settled: false,
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
        client.events.push(Event::Session {
            status: SessionStatus::Offline,
            session: None,
            level: None,
        });
        Ok(client)
    }

    /// The generator for a recipe, with the field this client holds when the
    /// recipe asks for one. A UI that wants other ground hands it over first.
    fn generator_for(&self, recipe: Recipe) -> Result<Generator, worldgen::RecipeError> {
        match (Generator::required_field(&recipe), self.field.clone()) {
            (Some(_), Some(field)) => Generator::with_field(recipe, field),
            _ => Generator::new(recipe),
        }
    }

    /// The ground a field world is shaped by, before its recipe arrives.
    /// Fails when the bytes are not a field; the world it belongs to is
    /// checked against its id when the recipe comes.
    pub fn set_field(&mut self, bytes: Vec<u8>) -> Result<(), worldgen::FieldError> {
        self.field = Some(Field::parse(bytes)?);
        Ok(())
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
            bloom: sane(effects.bloom, 0.0, 3.0, usual.bloom),
            bloom_threshold: sane(effects.bloom_threshold, 0.2, 8.0, usual.bloom_threshold),
            haze: sane(effects.haze, 0.0, 6.0, usual.haze),
            water_clarity: sane(effects.water_clarity, 0.25, 12.0, usual.water_clarity),
            parts_reach_m: sane(effects.parts_reach_m, 256.0, 8192.0, usual.parts_reach_m),
            ..effects
        };
        self.events.push(Event::EffectsChanged {
            effects: self.effects,
        });
    }

    pub fn recipe(&self) -> &Recipe {
        self.generator.recipe()
    }

    /// The size of the view in pixels: its aspect, and how tall a picture
    /// stands on it.
    pub fn set_view(&mut self, size: [u32; 2]) {
        let [width, height] = size.map(|n| n.max(1) as f32);
        self.aspect = width / height;
        self.view_height_px = height;
    }

    /// Stirs in randomness only a platform has (a clock, the browser). Without
    /// it every random choice is reproducible, which headless shots rely on.
    pub fn add_entropy(&mut self, bits: u64) {
        self.entropy ^= mix(bits);
    }

    /// Sets the clock: a server's on Welcome, a fixed one for reproducible
    /// headless renders.
    pub fn set_clock(&mut self, seconds: f64) {
        self.clock_s = seconds;
    }

    /// The body this world is printed at: its size, and everything in metres
    /// that follows from it. Frozen with the recipe.
    pub fn sphere(&self) -> topology::QuadSphere {
        self.generator.sphere()
    }

    /// The moon of the world: its size, and the grid what is built on it is
    /// cut by.
    pub fn moon(&self) -> topology::QuadSphere {
        self.generator.moon()
    }

    /// Stands the avatar at a point of any sector. Arrival, not travel.
    pub fn teleport(&mut self, point: SurfacePoint) {
        self.controller.teleport(point, &self.generator);
    }

    /// Local up where the avatar stands: out of the body it is on.
    pub fn up(&self) -> [f64; 3] {
        self.controller.up().to_array()
    }

    /// Where the avatar is and which way it looks, as a link carries it.
    pub fn here(&self) -> place::Pose {
        let grid = self.generator.sphere().blocks();
        // Every body is parametrised on the planet's sector grid, so the moon
        // has a column too: the one its radial passes through.
        let point = if self.controller.on_moon() {
            grid.surface_point(self.controller.radial().to_array())
        } else {
            self.controller.point()
        };
        place::Pose {
            on_moon: self.controller.on_moon(),
            column: grid.column_of(point),
            // Standing on the ground is what leaves the height out: the ground
            // is a function of the recipe, so it says itself.
            h: (!self.controller.grounded())
                .then(|| (self.controller.height_m() / topology::BLOCK_M) as i16),
            bearing_deg: topology::bearing_deg(
                self.controller.up().to_array(),
                self.controller.view().to_array(),
            ),
            pitch_deg: Some(self.controller.pitch().to_degrees()),
            chars: topology::CODE_MAX,
        }
    }

    /// Stands the avatar where a pose says and looks the way it says:
    /// `"4-K7M42Q"`, or `"m4-K7M42Q@40,180,-5"`.
    ///
    /// A code shorter than full precision names a box, so the middle of that
    /// box is where you land: a person who quotes four characters means the
    /// neighbourhood, and the middle of it is the least surprising answer.
    pub fn go_to(&mut self, text: &str) -> Result<(), String> {
        let moon = self.moon_position();
        travel(
            &mut self.controller,
            &self.generator,
            moon,
            &mut self.events,
            text,
        )
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
            Command::SetRecipe { recipe } => match self.generator_for(recipe) {
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
            Command::GoTo { place } => {
                if let Err(message) = self.go_to(&place) {
                    self.events.push(Event::Rejected { message });
                }
            }
            Command::SetName { name } => {
                self.session.rename(&name);
                self.wanted_name = name;
            }
            Command::PutDown => {
                let (plugins, lent) = self.lend();
                plugins.put_down(lent);
            }
        }
    }

    /// Whether a plugin has the pointer, a tool in hand: a shell hands the
    /// pointer's button over as [`Key::Use`] then, and keeps the pointer free
    /// to aim with.
    pub fn pointing(&self) -> bool {
        self.pointer.is_some()
    }

    /// Whether a plugin that is on in this world asks for a key of the
    /// keyboard, by the name the web gives it, with what is held with it. A
    /// shell hands such a key to [`Input::code`] and keeps it from whatever
    /// else would take it.
    pub fn asks(&self, code: &str, chord: Chord) -> bool {
        self.plugins.asks(code, chord)
    }

    /// The cells of the world, to read.
    pub fn cells(&self) -> &Cells {
        &self.cells
    }

    /// The plugins, and what of the core their host lends them for a call.
    fn lend(&mut self) -> (&mut Plugins, Lent<'_>) {
        let lent = Lent {
            session: &mut self.session,
            events: &mut self.events,
            generator: &self.generator,
            cells: &mut self.cells,
            parts: &mut self.parts,
            history: &mut self.history,
            controller: &mut self.controller,
            level: self.level,
            pointer: &mut self.pointer,
            passage: &mut self.passage,
            clock_s: self.clock_s,
        };
        (&mut self.plugins, lent)
    }

    /// A host for a plugin held outside this client: what a plugin's own
    /// tests stand on.
    pub fn host(&mut self, plugin: &'static str) -> Host<'_> {
        self.lend().1.host(plugin)
    }

    /// Stands in for a world's word where there is no world to say it: this
    /// session has `level`, and every plugin plugged in is on at the version
    /// it holds. For a headless picture, a bench and a test. A person's
    /// client hears both from a world alone (DECISIONS 104).
    pub fn rehearse(&mut self, level: Level) {
        self.level = Some(level);
        let plugins = self.plugins.rehearse();
        self.events.push(Event::Statement { plugins });
    }

    /// Commands arriving as JSON over the seam. Bad JSON becomes a
    /// [`Event::Rejected`], never a panic: the other side is a UI. A `type`
    /// with a dot in it is a plugin's, `chat.say`, and goes to that plugin.
    pub fn command_json(&mut self, json: &str) {
        let mut body = match serde_json::from_str::<serde_json::Value>(json) {
            Ok(body) => body,
            Err(error) => {
                return self.events.push(Event::Rejected {
                    message: error.to_string(),
                });
            }
        };
        let named = body
            .get("type")
            .and_then(serde_json::Value::as_str)
            .and_then(|name| name.split_once('.'))
            .map(|(plugin, kind)| (plugin.to_owned(), kind.to_owned()));
        if let Some((plugin, kind)) = named {
            if let Some(fields) = body.as_object_mut() {
                fields.remove("type");
            }
            return self.plugin_command(&plugin, &kind, body);
        }
        match serde_json::from_value::<Command>(body) {
            Ok(command) => self.command(command),
            Err(error) => self.events.push(Event::Rejected {
                message: error.to_string(),
            }),
        }
    }

    /// Plugs in a plugin's client half. A shell does it once, before the
    /// link opens, for each plugin its version carries. It stays off until a
    /// world says it is on.
    pub fn plug(&mut self, plugin: Box<dyn Plugin>) {
        self.plugins.plug(plugin);
    }

    /// A command for a plugin: `kind` is its name without the plugin's, and
    /// `body` the rest of its JSON. Refused when no plugin of that name is on
    /// in this world.
    pub fn plugin_command(&mut self, plugin: &str, kind: &str, body: serde_json::Value) {
        let (plugins, lent) = self.lend();
        if !plugins.command(plugin, kind, body, lent) {
            self.events.push(Event::Rejected {
                message: format!("`{plugin}.{kind}`: no plugin `{plugin}` is on in this world"),
            });
        }
    }

    /// Takes the world's statement of which plugins are on, and says it to
    /// the front end.
    fn spoken(&mut self, spoken: &[protocol::Plugin]) {
        let (plugins, lent) = self.lend();
        let (plugins, apart) = plugins.speak(spoken, lent);
        for message in apart {
            self.events.push(Event::Rejected { message });
        }
        self.events.push(Event::Statement { plugins });
    }

    pub fn drain_events(&mut self) -> Vec<Event> {
        core::mem::take(&mut self.events)
    }

    /// Files the client wants. The shell fetches each one and answers with
    /// [`Client::asset_loaded`].
    pub fn drain_asset_requests(&mut self) -> Vec<AssetRequest> {
        for hash in self.parts.wanted() {
            if let Some(name) = part::Shape::Vox(hash).asset() {
                self.requests.ask(name, Purpose::Model(hash));
            }
        }
        self.requests.drain()
    }

    /// The answer to an [`AssetRequest`] for a picture: its pixels, or why
    /// there are none. A level that fails is logged and asked for again
    /// when wanted.
    pub fn picture_loaded(&mut self, id: u64, image: Result<scene::Image, String>) {
        let Some(Purpose::Picture(content)) = self.requests.answer(id) else {
            return;
        };
        if let Err(message) = self.pictures.arrived(content, image) {
            self.events.push(Event::Rejected {
                message: format!("picture {}: {message}", content.hex()),
            });
        }
    }

    /// The answer to an [`AssetRequest`]: the bytes, or why there are none.
    /// A bad or missing file is logged as a rejection; the box figure and the
    /// rest pose are the fallbacks, so nothing breaks.
    pub fn asset_loaded(&mut self, id: u64, bytes: Result<Vec<u8>, String>) {
        let Some(purpose) = self.requests.answer(id) else {
            return;
        };
        let outcome = bytes.and_then(|bytes| match &purpose {
            Purpose::Manifest => serde_json::from_slice::<Manifest>(&bytes)
                .map(|manifest| self.adopt(manifest))
                .map_err(|e| e.to_string()),
            Purpose::Avatar(path) => {
                let avatar = avatar::Avatar::from_vrm(&bytes).map_err(|e| e.to_string());
                self.wardrobe.arrived(path, avatar).map(|_| ())
            }
            Purpose::Clip(gait) => avatar::Clip::from_glb(&bytes)
                .map(|clip| {
                    self.clips.insert(*gait, clip);
                })
                .map_err(|e| e.to_string()),
            Purpose::Model(hash) => vox::parse(&bytes)
                .map(|model| self.parts.model_arrived(*hash, model))
                .map_err(|e| format!("{e:?}")),
            Purpose::Picture(id) => Err(format!("{}: a picture is answered as pixels", id.hex())),
        });
        if let Err(message) = outcome {
            self.events.push(Event::Rejected {
                message: format!("{purpose:?}: {message}"),
            });
            if let Purpose::Avatar(path) = &purpose {
                self.fall_back_from(&path.clone());
            }
        }
        self.dress_all();
    }

    /// Asks for the avatar at an asset reference. Every way of choosing an
    /// avatar ends here, whether the reference came from the manifest, a
    /// cookie or, later, a user's own uploads.
    fn wear(&mut self, path: String) {
        self.wanted_avatar = Some(path.clone());
        self.session.wear(&path);
        self.dress_all();
    }

    /// The default avatar the manifest names, if it names one.
    fn default_avatar(&self) -> Option<String> {
        self.manifest
            .as_ref()
            .and_then(|m| m.default_avatar.clone())
    }

    /// A file at `path` is not an avatar: every body that wanted it wears the
    /// default instead. The default itself failing, or there being none,
    /// leaves the box figure.
    fn fall_back_from(&mut self, path: &str) {
        let default = self.default_avatar().filter(|d| d != path);
        if self.wanted_avatar.as_deref() == Some(path) {
            match &default {
                Some(default) => self.wear(default.clone()),
                None => self.wanted_avatar = None,
            }
        }
        for peer in self.peers.iter_mut().filter(|p| p.avatar == path) {
            peer.avatar = default.clone().unwrap_or_default();
        }
    }

    /// Puts every body in what it wants, from what has landed, and asks for
    /// what has not. A body whose file is still on its way stays as it is:
    /// the box figure, or what it wore before.
    fn dress_all(&mut self) {
        let default = self.default_avatar();
        let choose = |wanted: &str| -> Option<String> {
            if wanted.is_empty() {
                default.clone()
            } else {
                Some(wanted.to_owned())
            }
        };
        if let Some(wanted) = self.wanted_avatar.clone().and_then(|w| choose(&w))
            && dress(
                &mut self.figure,
                &wanted,
                &mut self.wardrobe,
                &mut self.requests,
            )
        {
            self.events.push(Event::AvatarChanged { path: wanted });
        }
        for peer in self.peers.iter_mut() {
            if let Some(wanted) = choose(&peer.avatar) {
                dress(
                    &mut peer.figure,
                    &wanted,
                    &mut self.wardrobe,
                    &mut self.requests,
                );
            }
        }
        let worn = self
            .figure
            .mesh()
            .into_iter()
            .chain(self.peers.iter().filter_map(|p| p.figure.mesh()));
        self.wardrobe.prune(worn);
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
        self.wardrobe.drain_changes()
    }

    /// Pictures to upload or drop, in the order the client produced them.
    pub fn drain_picture_changes(&mut self) -> Vec<PictureChange> {
        self.pictures.drain_changes()
    }

    /// The shell opened the link to the world server: the client says hello.
    /// Which world is in the socket's address; who this is was settled by
    /// the key that rode along with it.
    pub fn link_opened(&mut self) {
        self.session.opened(
            self.wanted_avatar.as_deref().unwrap_or(""),
            &self.wanted_name,
        );
        self.events.push(Event::Session {
            status: SessionStatus::Connecting,
            session: None,
            level: self.level,
        });
    }

    /// The link is gone, from either side.
    pub fn link_closed(&mut self) {
        self.session.closed();
        self.went_offline();
    }

    /// One frame from the server.
    pub fn receive(&mut self, frame: &[u8]) {
        use protocol::server_message::Message;
        let message = match protocol::decode::<protocol::ServerMessage>(frame) {
            Ok(message) => message,
            Err(error) => {
                self.events.push(Event::Rejected {
                    message: format!("frame from the server: {error}"),
                });
                return;
            }
        };
        self.session.heard();
        match message.message {
            Some(Message::Welcome(welcome)) => {
                if !self.same_world(welcome.recipe.as_ref()) {
                    self.events.push(Event::Rejected {
                        message: "the server holds another recipe for this world".into(),
                    });
                    self.session.close();
                    self.went_offline();
                    return;
                }
                self.session.welcomed(welcome.session);
                // The world's clock is the server's from here on.
                self.set_clock(welcome.clock_ms as f64 / 1000.0);
                self.level = Some(Level::from_wire(welcome.level));
                // The world keeps the cells from here on.
                self.cells.link(welcome.session);
                self.parts.link(welcome.session);
                self.history.clear();
                for peer in &welcome.peers {
                    self.peers.add(peer, &self.generator);
                }
                self.events.push(Event::Session {
                    status: SessionStatus::Online,
                    session: Some(welcome.session),
                    level: self.level,
                });
                self.spoken(&welcome.plugins);
                self.peers_changed();
            }
            Some(Message::Joined(joined)) => {
                if let Some(peer) = &joined.peer {
                    self.peers.add(peer, &self.generator);
                    self.peers_changed();
                }
            }
            Some(Message::Left(left)) => {
                self.peers.remove(left.session);
                self.peers_changed();
            }
            Some(Message::Stances(stances)) => {
                let own = match self.session.link() {
                    session::Link::Online { session } => session,
                    _ => 0,
                };
                for moved in &stances.moved {
                    if let Some(stance) = &moved.stance
                        && moved.session != own
                    {
                        self.peers.moved(moved.session, stance, &self.generator);
                    }
                }
            }
            Some(Message::Wearing(wearing)) => {
                self.peers.wearing(wearing.session, &wearing.avatar);
                self.dress_all();
            }
            Some(Message::Renamed(renamed)) => {
                self.peers.renamed(renamed.session, &renamed.name);
                self.peers_changed();
            }
            // The cells are the core's, and their events come under their
            // name as a plugin's do under its own.
            Some(Message::Envelope(envelope)) if envelope.plugin == protocol::cells::OWNER => {
                self.cells
                    .receive(&self.generator, &envelope.kind, &envelope.payload);
            }
            Some(Message::Envelope(envelope)) if envelope.plugin == protocol::parts::OWNER => {
                self.parts
                    .receive(&self.generator, &envelope.kind, &envelope.payload);
            }
            Some(Message::Envelope(envelope)) => {
                let (plugins, lent) = self.lend();
                plugins.receive(&envelope, lent);
            }
            Some(Message::Answer(answer)) => {
                if !answer.code.is_empty() {
                    self.history.refused(answer.id);
                }
                let cells = self
                    .cells
                    .answered(&self.generator, answer.id, &answer.code);
                let parts = !cells && self.parts.answered(answer.id, &answer.code);
                if (cells || parts) && !answer.code.is_empty() {
                    let what = if cells { "cells" } else { "parts" };
                    self.events.push(Event::Rejected {
                        message: format!(
                            "the world refused a change to the {what}: {}",
                            answer.code
                        ),
                    });
                }
            }
            Some(Message::Plugins(plugins)) => self.spoken(&plugins.plugins),
            Some(Message::Refused(refused)) => {
                self.events.push(Event::Rejected {
                    message: format!("refused by the server: {}", refused.reason),
                });
                self.session.closed();
                self.went_offline();
            }
            None => {}
        }
    }

    /// Whether the server's recipe is the one this client stands in. A page
    /// sets the recipe from the same row the server reads, so a difference
    /// is a stale page, and peers placed by another recipe would stand on
    /// the wrong ground.
    fn same_world(&self, recipe: Option<&protocol::Recipe>) -> bool {
        let Some(recipe) = recipe else {
            return false;
        };
        let own = self.recipe();
        let params = if recipe.params_json.trim().is_empty() {
            Ok(Params::default())
        } else {
            serde_json::from_str::<Params>(&recipe.params_json)
        };
        recipe.seed == worldgen::format_seed(own.seed)
            && recipe.generator_version == own.generator_version
            && params.is_ok_and(|params| params == own.params)
    }

    fn peers_changed(&mut self) {
        self.dress_all();
        self.events.push(Event::Peers {
            peers: self.peers.infos(),
        });
    }

    fn went_offline(&mut self) {
        // What the world kept went with it.
        self.cells.unlink();
        self.parts.unlink();
        self.history.clear();
        self.peers.clear();
        self.dress_all();
        self.events.push(Event::Session {
            status: SessionStatus::Offline,
            session: None,
            level: self.level,
        });
        let (plugins, lent) = self.lend();
        let plugins = plugins.hush(lent);
        self.events.push(Event::Statement { plugins });
        self.events.push(Event::Peers { peers: Vec::new() });
    }

    /// What the shell sends on the link, in order. Empty while offline.
    pub fn drain_outbound(&mut self) -> Vec<Outbound> {
        for ask in self.cells.drain_asks() {
            self.session
                .ask(protocol::cells::OWNER, ask.kind, ask.payload, ask.id);
        }
        for ask in self.parts.drain_asks() {
            self.session
                .ask(protocol::parts::OWNER, ask.kind, ask.payload, ask.id);
        }
        self.session.drain_outbound()
    }

    /// Where this body is and how it moves, for the wire.
    fn stance(&self) -> protocol::Stance {
        let controller = &self.controller;
        let grid = self.generator.sphere().blocks();
        let point = if controller.on_moon() {
            grid.surface_point(controller.radial().to_array())
        } else {
            controller.point()
        };
        let facing = controller.facing();
        let gait = Gait::of(controller);
        let body = if controller.on_moon() {
            protocol::Body::Moon
        } else {
            protocol::Body::Planet
        };
        protocol::Stance {
            body: body.into(),
            sector: point.sector.index() as u32,
            u: point.u as f32,
            v: point.v as f32,
            height_m: controller.height_m() as f32,
            facing_x: facing.x as f32,
            facing_y: facing.y as f32,
            facing_z: facing.z as f32,
            gait: gait.wire().into(),
            speed_mps: figure::speed_for(gait, controller.ground_mps(), controller.speed_mps())
                as f32,
            sprint: controller.sprinting(),
        }
    }

    /// Volume mesh uploads and removals since the last call, in order.
    pub fn drain_volume_changes(&mut self) -> Vec<VolumeChange> {
        self.cells.drain_changes()
    }

    /// The meshes of models to upload and drop, in order.
    pub fn drain_model_changes(&mut self) -> Vec<ModelChange> {
        self.parts.drain_changes()
    }

    /// The parts of the world, to read: what stands where.
    pub fn parts(&self) -> &Parts {
        &self.parts
    }

    /// The ground of the world, to read.
    pub fn generator(&self) -> &Generator {
        &self.generator
    }

    /// What a hand changed, to read: whether there is a change to take
    /// back, and one to put back.
    pub fn history(&self) -> &History {
        &self.history
    }

    /// Takes back the last change a hand made here, whatever owner it was
    /// of: the owner takes its own back, here or in the world.
    pub fn take_back(&mut self) {
        match self.history.take_back() {
            Some(Owner::Cells) => self.cells.take_back(&self.generator),
            Some(Owner::Parts) => self.parts.restore(&self.generator, true),
            None => {}
        }
    }

    /// Puts back the last change taken back.
    pub fn put_back(&mut self) {
        match self.history.put_back() {
            Some(Owner::Cells) => self.cells.put_back(&self.generator),
            Some(Owner::Parts) => self.parts.restore(&self.generator, false),
            None => {}
        }
    }

    pub fn drain_terrain_changes(&mut self) -> Vec<TerrainChange> {
        let mut changes = self.terrain.drain_changes();
        changes.extend(self.moon_terrain.drain_changes());
        changes
    }

    /// Advances the simulation by `dt` seconds and returns the frame to draw.
    pub fn update(&mut self, dt: f64, input: &mut Input) -> Frame {
        let taken = input.take_frame();
        let (look, zoom) = (taken.look, taken.zoom);
        self.entropy = self.entropy.wrapping_add(dt.to_bits()).rotate_left(7);
        for key in taken.pressed {
            match key {
                Key::ToggleMode => self.set_mode(match self.controller.mode {
                    Mode::Walk => Mode::Fly,
                    Mode::Fly => Mode::Walk,
                }),
                Key::NewSeed => {
                    // A new seed over the same ground: a field world keeps its
                    // coastlines and gets new mountains under them.
                    let recipe = Recipe {
                        params: self.recipe().params,
                        ..Recipe::new(mix(self.recipe().seed ^ self.entropy))
                    };
                    self.command(Command::SetRecipe { recipe });
                }
                Key::NextAvatar => self.next_avatar(),
                Key::Undo => self.take_back(),
                Key::Redo => self.put_back(),
                _ => {}
            }
        }
        // A plugin hears the keys it asked for by name.
        let (plugins, lent) = self.lend();
        plugins.keys(&taken.codes, lent);

        // The clock takes every second, a stalled tab's too, so it keeps
        // pace with the server's. Long frames must not tunnel the avatar
        // anywhere: the body takes a tenth at most.
        self.clock_s += dt.max(0.0);
        let dt = dt.clamp(0.0, 0.1);
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
        self.cells.orbit(moon);
        self.parts.orbit(moon);
        self.controller.update(
            dt,
            wish,
            &self.generator,
            Built {
                cells: &self.cells,
                parts: &self.parts,
            },
        );
        self.passage_turn();
        let motion = self.motion();
        self.figure.update(dt, &motion, &self.clips);
        self.peers.update(dt, moon, &self.clips);
        let stance = self.stance();
        if self.session.tick(dt, stance) {
            self.events.push(Event::Rejected {
                message: "the server went silent".into(),
            });
            self.session.close();
            self.went_offline();
        }
        self.stats(dt);

        let camera = self.controller.camera(&self.generator);
        self.anchors(&camera);
        self.labels(&camera);
        let eye = Eye {
            position: camera.position,
            rotation: camera.rotation,
            fov_y: f64::from(camera.fov_y),
            aspect: f64::from(self.aspect),
            // A captured pointer aims through the middle of the view.
            pointer: input.pointer.unwrap_or([0.5, 0.5]).map(f64::from),
        };
        // Each plugin's turn, then the cells drawn as the turns left them,
        // and kept in step with what the world holds near the body.
        let (plugins, lent) = self.lend();
        plugins.turn(&eye, input, (taken.interrupted, taken.inspected), lent);
        self.cells.update(eye.position);
        self.cells
            .look(&self.generator, self.controller.held_by(), dt);
        self.parts
            .look(&self.generator, self.controller.held_by(), dt);
        let through = self.through(&camera);
        let (patches, seen) = self.stream(&camera, through.as_ref(), Terrain::update);
        // Said on the way in, never while it holds: a front end lifts its
        // veil on it, and hears it again after a leap or a new recipe.
        let settled = self.terrain.settled()
            && self.moon_terrain.settled()
            && self.cells.settled()
            && !self.plugins.busy();
        if settled && !self.settled {
            self.events.push(Event::Settled);
        }
        self.settled = settled;
        let pictures = self.pictures_drawn(&camera);
        self.frame(camera, patches, pictures, through.zip(seen))
    }

    /// Where every head in view lands on the screen, own body first, with
    /// the renderer's own projection: reversed infinite depth has no far
    /// plane, so only what is behind the camera or off the sides is left out.
    fn anchors(&mut self, camera: &scene::Camera) {
        let own = match self.session.link() {
            session::Link::Online { session } => Some(session),
            _ => None,
        };
        let moon = self.moon_position();
        let own_head = own.map(|session| {
            let up = self.controller.body_basis().y_axis;
            (
                session,
                self.controller.drawn_position() + up * self.figure.label_m(),
            )
        });
        let projection = Projection::of(camera, self.aspect);
        let anchors: Vec<Anchor> = own_head
            .into_iter()
            .chain(
                own.is_some()
                    .then(|| self.peers.heads(moon))
                    .into_iter()
                    .flatten(),
            )
            .filter_map(|(session, head)| {
                let (x, y, distance_m) = projection.project(head)?;
                Some(Anchor {
                    session,
                    x,
                    y,
                    distance_m,
                })
            })
            .collect();
        if anchors.is_empty() && !self.had_anchors {
            return;
        }
        self.had_anchors = !anchors.is_empty();
        self.events.push(Event::Anchors { anchors });
    }

    /// Where every landmark in view and within reach of a label lands on
    /// the screen, the point over its star, as heads do.
    fn labels(&mut self, camera: &scene::Camera) {
        let projection = Projection::of(camera, self.aspect);
        let body = self.controller.held_by();
        let labels: Vec<Label> = self
            .parts
            .all()
            .filter_map(|(id, seat, part)| {
                let Shape::Landmark(name) = part.shape else {
                    return None;
                };
                if parts::away_m(&self.generator, seat, &part.pose, body) > LABEL_M {
                    return None;
                }
                let world = self.parts.transform(&self.generator, id)?;
                let top = world.transform_point3(DVec3::new(0.0, 0.0, 1.0));
                let (x, y, distance_m) = projection.project(top)?;
                Some(Label {
                    id: id.0,
                    text: name.to_string(),
                    x,
                    y,
                    distance_m,
                })
            })
            .collect();
        if labels.is_empty() && !self.had_labels {
            return;
        }
        self.had_labels = !labels.is_empty();
        self.events.push(Event::Labels { labels });
    }

    /// The frame as it would look once streaming caught up. Blocks until every
    /// patch is built: for headless renders only.
    pub fn settled_frame(&mut self) -> Frame {
        // The moon is where the clock says now, for whoever stands on it and
        // for what is built there, as for its ground.
        let moon = self.moon_position();
        self.controller.set_moon(controller::MoonBody {
            center: moon,
            radius_m: MOON_RADIUS_M,
        });
        self.cells.orbit(moon);
        self.parts.orbit(moon);
        let eye = self.controller.camera(&self.generator).position;
        let (plugins, lent) = self.lend();
        plugins.settle(lent);
        self.cells.settle(eye);
        let camera = self.controller.camera(&self.generator);
        let through = self.through(&camera);
        let (patches, seen) = self.stream(&camera, through.as_ref(), Terrain::settle);
        let pictures = self.pictures_drawn(&camera);
        self.frame(camera, patches, pictures, through.zip(seen))
    }

    /// The body's motion as the figure shows it: falling, while it passes
    /// through a portal.
    fn motion(&self) -> Motion {
        let mut motion = Motion::of(&self.controller);
        if self.passage.is_some() {
            motion.gait = Gait::Fall;
        }
        motion
    }

    /// A passage's turn: the body goes through the portal [`WARP_IN_S`]
    /// after it fell in, and the passage ends [`WARP_OUT_S`] after that.
    /// A place that cannot be reached leaves the body where it is, said.
    fn passage_turn(&mut self) {
        let Some(passage) = &mut self.passage else {
            return;
        };
        let elapsed = self.clock_s - passage.started_s;
        if !passage.through && elapsed >= WARP_IN_S {
            passage.through = true;
            let place = passage.place.clone();
            if let Err(message) = self.go_to(&place) {
                self.events.push(Event::Rejected { message });
                self.passage = None;
            }
        } else if elapsed >= WARP_IN_S + WARP_OUT_S {
            self.passage = None;
        }
    }

    /// How charged the body is by its passage through a portal, 0 to 1:
    /// white as it goes through, its own colour again as it lands.
    fn charge(&self) -> f32 {
        let Some(passage) = &self.passage else {
            return 0.0;
        };
        let elapsed = self.clock_s - passage.started_s;
        let charge = match passage.through {
            false => elapsed / WARP_IN_S,
            true => 1.0 - (elapsed - WARP_IN_S) / WARP_OUT_S,
        };
        charge.clamp(0.0, 1.0) as f32
    }

    /// The warp of the picture this frame: into the portal the body is
    /// falling into, out of the body where it landed.
    fn warp(&self, camera: &scene::Camera) -> Option<scene::Warp> {
        let passage = self.passage.as_ref()?;
        let elapsed = self.clock_s - passage.started_s;
        let projection = Projection::of(camera, self.aspect);
        let on_screen = |point: DVec3| {
            projection
                .project(point)
                .map_or([0.5, 0.5], |(x, y, _)| [x, y])
        };
        Some(match passage.through {
            false => {
                let pane = self
                    .parts
                    .transform(&self.generator, passage.portal)
                    .map(|ring| ring.transform_point3(DVec3::new(0.0, 0.0, 0.5)));
                scene::Warp {
                    centre: pane.map_or([0.5, 0.5], on_screen),
                    phase: ((elapsed / WARP_IN_S).min(1.0) - 1.0) as f32,
                }
            }
            true => {
                let waist = self.controller.drawn_position() + self.controller.up() * 0.9;
                scene::Warp {
                    centre: on_screen(waist),
                    phase: (((elapsed - WARP_IN_S) / WARP_OUT_S).clamp(0.0, 1.0)) as f32,
                }
            }
        })
    }

    /// The portal the view is seen through this frame, and the eye past it:
    /// the nearest portal within [`THROUGH_M`] that leads to a landmark
    /// whose place is known here, and the camera carried from the portal's
    /// frame to the landmark's, as a body that walked through would be. A
    /// landmark this client does not hold is asked of the chart.
    fn through(&mut self, camera: &scene::Camera) -> Option<(PartId, scene::Camera)> {
        let body = self.controller.held_by();
        let (_, id, to) = self
            .parts
            .all()
            .filter_map(|(id, seat, part)| {
                let Shape::Portal(Portal { to: Some(to), .. }) = part.shape else {
                    return None;
                };
                let away_m = parts::away_m(&self.generator, seat, &part.pose, body);
                (away_m <= THROUGH_M).then_some((away_m, id, to))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0))?;
        let ring = self.parts.transform(&self.generator, id)?;
        let Some(star) = self.parts.landmark_transform(&self.generator, to) else {
            self.parts.chart_for(self.clock_s);
            return None;
        };
        // The pane is the ring's `y = 0` plane and its heading `+y`; the star
        // lies on the ground with its heading `+y` too: frame to frame, the
        // rigid part alone, since a portal is a door, not a lens.
        let carried = rigid(&star) * rigid(&ring).inverse();
        Some((
            id,
            scene::Camera {
                position: carried.transform_point3(camera.position),
                rotation: DQuat::from_mat3(&carried.matrix3) * camera.rotation,
                ..*camera
            },
        ))
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
        through: Option<&(PartId, scene::Camera)>,
        select: Select,
    ) -> (Vec<scene::PatchDraw>, Option<Vec<scene::PatchDraw>>) {
        let moon = self.moon_position();
        // The view's eye first, then the eye past a portal, which sees at a
        // quarter of the view's height and asks for a quarter of its detail.
        let eyes: Vec<Viewpoint> = core::iter::once(Viewpoint {
            camera: *camera,
            coarseness: 1.0,
        })
        .chain(through.map(|(_, camera)| Viewpoint {
            camera: *camera,
            coarseness: 4.0,
        }))
        .collect();
        let from_moon: Vec<Viewpoint> = eyes
            .iter()
            .map(|eye| Viewpoint {
                camera: scene::Camera {
                    position: eye.camera.position - moon,
                    ..eye.camera
                },
                coarseness: eye.coarseness,
            })
            .collect();
        // Grass grows again where what is built over it changed.
        for (middle, angle) in self.cells.drain_touched() {
            self.terrain.regrow(middle, angle);
        }
        let build = &self.cells;
        let on_planet = select(
            &mut self.terrain,
            &self.generator,
            &|sector, column, heights| build.covered(sector, column, heights),
            &eyes,
            self.aspect,
        );
        // No grass grows on the moon for what is built there to cover.
        let on_moon = select(
            &mut self.moon_terrain,
            &self.generator,
            &|_, _, _| false,
            &from_moon,
            self.aspect,
        );
        let at = |body_center: DVec3| move |id| scene::PatchDraw { id, body_center };
        let mut lists = on_planet.into_iter().zip(on_moon).map(|(planet, on_moon)| {
            let planet = planet.into_iter().map(at(DVec3::ZERO));
            planet
                .chain(on_moon.into_iter().map(at(moon)))
                .collect::<Vec<_>>()
        });
        let view = lists.next().expect("the view's eye");
        (view, lists.next())
    }

    /// Every picture placed, at the level it is seen at from the camera,
    /// and the frame closed for the pictures: what went unseen is dropped.
    fn pictures_drawn(&mut self, camera: &scene::Camera) -> Vec<scene::PictureDraw> {
        let screen = Screen {
            camera: *camera,
            aspect: f64::from(self.aspect),
            height_px: f64::from(self.view_height_px),
        };
        let drawn = self.parts.pictures(
            &self.generator,
            self.controller.held_by(),
            f64::from(self.effects.parts_reach_m),
            &mut self.pictures,
            &screen,
            &mut self.requests,
        );
        self.pictures.end_frame();
        drawn
    }

    fn frame(
        &self,
        camera: scene::Camera,
        patches: Vec<scene::PatchDraw>,
        pictures: Vec<scene::PictureDraw>,
        through: Option<((PartId, scene::Camera), Vec<scene::PatchDraw>)>,
    ) -> Frame {
        let angle = self.noon_offset + self.clock_s / DAY_S * core::f64::consts::TAU;
        let sun = DVec3::new(angle.cos(), 0.35, angle.sin()).normalize();

        // Every body, the player's first: an avatar when its file has
        // landed, the box figure until then.
        let mut boxes = Vec::new();
        let mut skinned = Vec::new();
        let charge = self.charge();
        let bodies = core::iter::once((&self.figure, self.motion(), charge)).chain(
            self.peers
                .bodies(self.moon_position())
                .map(|(figure, motion)| (figure, motion, 0.0)),
        );
        for (figure, motion, shine) in bodies {
            match figure.instance(&motion, &self.clips) {
                Some(instance) => skinned.push(scene::SkinnedInstance { shine, ..instance }),
                None => boxes.extend(
                    box_figure::parts(&motion)
                        .into_iter()
                        .map(|part| scene::BoxPart { shine, ..part }),
                ),
            }
        }
        Frame {
            camera,
            sun_direction: sun.as_vec3(),
            moon: scene::Moon {
                position: self.moon_position(),
                radius_m: MOON_RADIUS_M,
            },
            planet_radius_m: self.generator.sphere().radius_m(),
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
                start: self.controller.drawn_position() + self.controller.up() * 0.2,
                end: self.controller.drawn_position() + self.controller.up() * 1.5,
                radius_m: 0.65,
            },
            boxes,
            skinned,
            volumes: self.cells.drawn(),
            glass: self.cells.glazed(),
            lamps: {
                let mut lamps = self.cells.lamps();
                lamps.extend(self.parts.lamps(&self.generator));
                lamps
            },
            models: self.parts.drawn(
                &self.generator,
                self.controller.held_by(),
                f64::from(self.effects.parts_reach_m),
                self.clock_s,
            ),
            ghost: self.cells.ghost(),
            guides: self.cells.guides_drawn(),
            pictures,
            panes: self.parts.panes(
                &self.generator,
                self.controller.held_by(),
                f64::from(self.effects.parts_reach_m),
                through.as_ref().map(|((id, _), _)| *id),
            ),
            through: through.map(|((_, camera), patches)| scene::Through { camera, patches }),
            warp: self.warp(&camera),
        }
    }

    fn set_mode(&mut self, mode: Mode) {
        if self.controller.mode != mode {
            self.controller.mode = mode;
            self.events.push(Event::ModeChanged { mode });
        }
    }

    fn regenerate(&mut self, generator: Generator) {
        // Another recipe is another world, and the link was to this one. A
        // front end that changes worlds opens a new link afterwards.
        if self.session.link() != session::Link::Offline {
            self.session.close();
            self.went_offline();
        }
        let mode = self.controller.mode;
        // Turning a knob should leave you where you stood. But the ground
        // under a place belongs to the recipe, so the same address on new
        // ground can be open sea, and over a field of the Earth it usually
        // is: the planet is 71% water. Keep the place only while it is still
        // a place to stand, and let the world choose otherwise.
        let held = self.controller.point();
        let keep = !self.controller.on_moon()
            && self.controller.sphere() == generator.sphere()
            && standable(&generator, held);
        self.generator = generator;
        // What a plugin had in hand was for the old ground, and so were the
        // volumes that stood on it.
        let (plugins, lent) = self.lend();
        plugins.rest(lent);
        self.cells.clear();
        self.pictures.clear();
        if keep {
            // A flyer keeps its altitude, which `stand_at` lifts if the new
            // ground rose through it; a walker lands on whatever is there now.
            let height_m = (mode == Mode::Fly).then(|| self.controller.height_m());
            self.controller.stand_at(height_m, &self.generator);
        } else {
            self.controller = Controller::spawn(spawn_point(&self.generator), &self.generator);
        }
        self.controller.mode = mode;
        self.terrain.clear();
        self.moon_terrain.clear();
        // A new world starts its own clock.
        self.clock_s = 0.0;
        self.face_the_sun();
        self.events.push(Event::RecipeChanged {
            recipe: self.generator.recipe().clone(),
        });
    }

    /// Mid morning at the spawn when the clock reads zero. The spawn is the
    /// recipe's, so the offset is the world's too.
    fn face_the_sun(&mut self) {
        let up = self.controller.up();
        self.noon_offset = up.z.atan2(up.x) - 0.7;
    }

    fn stats(&mut self, dt: f64) {
        self.stats_timer_s += dt;
        self.frames_since_stats += 1;
        if self.stats_timer_s >= STATS_EVERY_S {
            let grid = self.generator.sphere().blocks();
            let pose = self.here();
            self.events.push(Event::Stats {
                fps: (f64::from(self.frames_since_stats) / self.stats_timer_s) as f32,
                altitude_m: self.controller.altitude_m(&self.generator),
                speed_mps: self.controller.speed_mps(),
                place: pose.place(grid),
                pose: pose.text(grid),
                bearing_deg: pose.bearing_deg,
            });
            self.stats_timer_s = 0.0;
            self.frames_since_stats = 0;
        }
    }
}

/// Whether a body could stand at `point`: dry ground, clear of the waves.
///
/// This is what a place carried from one recipe to another has to answer. An
/// address on its own says nothing about the ground, because the ground is
/// the recipe's, so the same characters can name a hill under one and the
/// middle of an ocean under the next.
fn standable(generator: &Generator, point: SurfacePoint) -> bool {
    let direction = generator.sphere().blocks().direction(point);
    dry(generator.sample(direction), generator.scale())
}

/// Ground clear of the waves. Heights are in reference metres, so the margin
/// scales with the body: a metre of clearance on the reference planet, less
/// on a smaller one.
fn dry(sample: Sample, scale: f64) -> bool {
    sample.height_m > scale
}

/// A place to stand: the first gentle, dry ground along a fixed search path
/// over the whole body. Deterministic per recipe, so a world always opens the
/// same.
///
/// The search is every sector, not just the first. A face of a world can be
/// all ocean, and on a field of the Earth one of them is most of the Pacific,
/// so a search that gave up after sector 0 spawned people hundreds of metres
/// under water. Sector 0 is still walked first, so a world that had an answer
/// there opens where it always did.
fn spawn_point(generator: &Generator) -> SurfacePoint {
    let grid = generator.sphere().blocks();
    let side = f64::from(grid.side());
    // Heights are written in reference metres and printed at the body's own
    // size (50), so what counts as dry ground scales with the world.
    let scale = generator.scale();
    // A coarse lattice ordered from a sector's centre outward.
    let mut cells: Vec<(i32, i32)> = (-12..=12)
        .flat_map(|i| (-12..=12).map(move |j| (i, j)))
        .collect();
    cells.sort_by_key(|(i, j)| i * i + j * j);
    // The highest ground seen anywhere, for a world with no gentle spot at
    // all: the shallowest water is the likeliest place to find a shoal, and
    // is a kinder answer than the middle of a sector.
    let mut highest: Option<(f64, SurfacePoint)> = None;
    for sector in Sector::ALL {
        for &(i, j) in &cells {
            let point = SurfacePoint::new(
                sector,
                side * (0.5 + f64::from(i) / 26.0),
                side * (0.5 + f64::from(j) / 26.0),
            );
            let sample = generator.sample(grid.direction(point));
            let gentle = dry(sample, scale) && sample.material != Material::Snow;
            if gentle && sample.height_m < 300.0 * scale {
                return point;
            }
            match highest {
                Some((height_m, _)) if height_m >= sample.height_m => {}
                _ => highest = Some((sample.height_m, point)),
            }
        }
    }
    // A water world, or one frozen solid: stand on the best there is.
    let (_, point) = highest.expect("the lattice has at least one cell");
    point
}

/// splitmix64 finalizer: spreads a counter into a seed.
fn mix(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

/// Puts one body in the avatar it wants, when the file has landed, and asks
/// for it otherwise. True when the body changed into it just now.
fn dress(
    figure: &mut Figure,
    wanted: &str,
    wardrobe: &mut Wardrobe,
    requests: &mut Requests,
) -> bool {
    match wardrobe.get(wanted) {
        Some(worn) if figure.mesh() != Some(worn.mesh) => {
            figure.wear(worn);
            true
        }
        Some(_) => false,
        None => {
            wardrobe.want(wanted, requests);
            false
        }
    }
}

/// Takes the body to a place, as [`Client::go_to`] and a plugin's
/// [`Host::go_to`] both do: the point first, because it is what the body
/// stands on and what the moon's radial is read from; then the body, then
/// the height, then the way of looking. Each step reads the one before it.
pub(crate) fn travel(
    controller: &mut Controller,
    generator: &Generator,
    moon: DVec3,
    events: &mut Vec<Event>,
    text: &str,
) -> Result<(), String> {
    let grid = generator.sphere().blocks();
    let pose = place::Pose::parse(grid, text)?;
    let found = topology::Place {
        column: pose.column,
        h: pose.h,
        chars: pose.chars,
    };
    let [su, sv] = found.span(grid).map(f64::from);
    let point = SurfacePoint::new(
        pose.column.sector,
        f64::from(pose.column.u) + su / 2.0,
        f64::from(pose.column.v) + sv / 2.0,
    );
    let mode = controller.mode;
    controller.teleport(point, generator);
    if pose.on_moon {
        controller.set_moon(controller::MoonBody {
            center: moon,
            radius_m: MOON_RADIUS_M,
        });
        let direction = DVec3::from(grid.direction(point));
        controller.stand_on_moon(direction, generator);
    }
    controller.stand_at(pose.h.map(|h| f64::from(h) * topology::BLOCK_M), generator);
    if let Some(bearing) = pose.bearing_deg {
        controller.face(bearing);
    }
    if let Some(pitch) = pose.pitch_deg {
        controller.set_pitch(pitch.to_radians());
    }
    // A height off the ground is a flight, and a front end that showed a
    // walker has to hear it: the panel says what the body does.
    if controller.mode != mode {
        events.push(Event::ModeChanged {
            mode: controller.mode,
        });
    }
    Ok(())
}

/// How far from the body a landmark's name is hung over it, metres: as far
/// as a nametag is read.
const LABEL_M: f64 = 120.0;

/// How a streamer picks the patches each eye draws: [`Terrain::update`]
/// within a frame's budget, or [`Terrain::settle`] until nothing is missing.
type Select = fn(&mut Terrain, &Generator, Cover, &[Viewpoint], f32) -> Vec<Vec<scene::PatchId>>;

/// How far from the body a portal is seen through, metres: a plot's width.
/// One at a time, the nearest.
const THROUGH_M: f64 = 32.0;

/// How long a body falls into a portal before it is through, and how long
/// the picture takes to burst out past it, seconds: a passage is under a
/// second, a blink of the world.
const WARP_IN_S: f64 = 0.35;
const WARP_OUT_S: f64 = 0.65;

/// A body's passage through a portal: it falls into the portal, the
/// picture drawn into it, goes through at [`WARP_IN_S`] and lands at the
/// place, the picture bursting out of it for [`WARP_OUT_S`].
pub(crate) struct Passage {
    place: String,
    portal: PartId,
    started_s: f64,
    /// Whether the body has gone through yet.
    through: bool,
}

impl Passage {
    pub(crate) fn new(place: String, portal: PartId, started_s: f64) -> Passage {
        Passage {
            place,
            portal,
            started_s,
            through: false,
        }
    }
}

/// A frame's rigid part: its rotation and translation, its scale taken out,
/// each axis made unit.
fn rigid(frame: &DAffine3) -> DAffine3 {
    let m = frame.matrix3;
    let axes = DMat3::from_cols(
        m.x_axis.normalize(),
        m.y_axis.normalize(),
        m.z_axis.normalize(),
    );
    DAffine3::from_mat3_translation(axes, frame.translation)
}

/// The renderer's own projection of a point of the world onto the screen:
/// reversed infinite depth has no far plane, so only what is behind the
/// camera or off the sides is left out.
struct Projection {
    inverse: DQuat,
    position: DVec3,
    focal: f64,
    aspect: f64,
    near: f64,
}

impl Projection {
    fn of(camera: &scene::Camera, aspect: f32) -> Projection {
        Projection {
            inverse: camera.rotation.inverse(),
            position: camera.position,
            focal: 1.0 / (f64::from(camera.fov_y) / 2.0).tan(),
            aspect: f64::from(aspect),
            near: f64::from(camera.near),
        }
    }

    /// Where a point lands: fractions of the viewport from the top left,
    /// and how far the point is. `None` behind the camera or off the sides,
    /// a little past the edge, so a label slides off instead of popping.
    fn project(&self, point: DVec3) -> Option<(f32, f32, f32)> {
        let local = self.inverse * (point - self.position);
        if local.z >= -self.near {
            return None;
        }
        let x = self.focal / self.aspect * local.x / -local.z;
        let y = self.focal * local.y / -local.z;
        if x.abs() > 1.2 || y.abs() > 1.2 {
            return None;
        }
        Some((
            ((x + 1.0) / 2.0) as f32,
            ((1.0 - y) / 2.0) as f32,
            local.length() as f32,
        ))
    }
}
