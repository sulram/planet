//! The avatar controller: walk with radial gravity, swim, fly like superman.
//!
//! State lives in address space (`SurfacePoint` + height), the one place where
//! blocks are unit cubes and where collision with the build layer will run.
//! World space enters only as velocities: a wish direction in metres is
//! converted to an address space delta through the local tangents, so speed
//! feels the same everywhere although blocks are not perfectly square.

use glam::{DMat3, DQuat, DVec3};
use scene::Camera;
use topology::{QuadSphere, SurfacePoint};
use worldgen::Generator;

use crate::collision::{self, BODY_M, Footing, STEP_M};
use crate::seam::Mode;

const WALK_MPS: f64 = 4.8;
/// Running is this much faster than walking; flying fast, than cruising.
const RUN_FACTOR: f64 = 2.5;
const FLY_SPRINT_FACTOR: f64 = 4.0;
const JUMP_MPS: f64 = 6.0;
const GRAVITY_MPS2: f64 = 14.0;
/// Flight speed grows with altitude so orbit is a short trip.
const FLY_MIN_MPS: f64 = 12.0;
const FLY_PER_ALTITUDE: f64 = 0.8;
const SWIM_MPS: f64 = 2.4;
const SWIM_SPRINT_FACTOR: f64 = 2.0;
/// Feet height while floating: the waterline sits at the waist, and a body
/// lying prone in a swim clip breaks the surface.
const FLOAT_M: f64 = -0.8;
/// Below this the swimmer is diving and steers with the camera pitch.
const DIVING_M: f64 = FLOAT_M - 0.35;
/// Looking down past this while swimming forward starts a dive, radians.
const DIVE_PITCH: f64 = -0.5;
/// Idle swimmers drift back up.
const BUOYANCY_MPS: f64 = 0.7;
/// The moon pulls a fifth as hard: the same legs jump five times as high.
const MOON_GRAVITY_MPS2: f64 = GRAVITY_MPS2 / 5.0;
/// The moon's sphere of influence, in moon radii from its centre. Inside it
/// the avatar is stored relative to the moon and rides its orbit. It is wide
/// on purpose: the hand-over happens where nothing can be seen of it. Leaving
/// takes a little more distance than entering, so the edge never flickers.
const MOON_SOI_ENTER: f64 = 4.0;
const MOON_SOI_EXIT: f64 = 4.4;
/// Between these distances (moon radii from its centre) down turns from the
/// planet to the moon, continuously.
const MOON_PULL_FAR: f64 = 3.0;
const MOON_PULL_NEAR: f64 = 1.3;
/// How fast the frame turns to gravity on foot, per second.
const WALK_ALIGN_PER_S: f64 = 4.0;
/// In flight the frame is the flyer's own. It only settles to the horizon
/// close to a surface: fully under `FLY_ALIGN_NEAR` body radii of clearance,
/// not at all over `FLY_ALIGN_FAR`.
const FLY_ALIGN_PER_S: f64 = 1.5;
const FLY_ALIGN_NEAR: f64 = 0.02;
const FLY_ALIGN_FAR: f64 = 0.30;
const EYE_M: f64 = 1.5;
const LOOK_RAD_PER_PX: f64 = 0.0025;
const PITCH_LIMIT: f64 = 1.45;
const TURN_PER_S: f64 = 12.0;

/// What the controller wants this frame, already free of key codes.
#[derive(Clone, Copy, Debug, Default)]
pub struct Wish {
    /// `[right, forward]`, each `-1..=1`.
    pub movement: [f32; 2],
    pub up: bool,
    pub down: bool,
    pub sprint: bool,
    pub look: [f32; 2],
    pub zoom: f32,
}

/// The celestial body whose frame of reference holds the avatar: what its
/// position is stored relative to. An avatar in the moon's sphere of influence
/// rides the moon's orbit without anyone moving it.
///
/// Which way is down is a separate matter, and continuous: see
/// [`Controller::gravity_up`].
#[derive(Clone, Copy, PartialEq, Debug)]
enum Site {
    /// Position is `point` + `height_m` over the datum sphere.
    Planet,
    /// Position is `direction` (unit, from the moon's centre) + `height_m`
    /// over the moon's datum sphere.
    Moon { direction: DVec3 },
}

/// Where the moon is this frame: centre in world space, and radius.
#[derive(Clone, Copy, Debug)]
pub struct MoonBody {
    pub center: DVec3,
    pub radius_m: f64,
}

#[derive(Clone, Debug)]
pub struct Controller {
    pub mode: Mode,
    /// The body this controller walks: its size, and the metres that follow.
    sphere: QuadSphere,
    point: SurfacePoint,
    /// Feet, metres above the datum sphere.
    height_m: f64,
    vertical_mps: f64,
    grounded: bool,
    /// Where the avatar faces: a unit tangent in world space.
    facing: DVec3,
    /// Where the camera looks, flattened on the tangent plane.
    view: DVec3,
    /// Camera pitch, radians, positive looks up.
    pitch: f64,
    /// Camera distance behind the avatar, metres.
    boom_m: f64,
    /// Speed over the last step, metres per second.
    speed_mps: f64,
    /// The part of it along the ground.
    ground_mps: f64,
    /// Distance walked, for the walk cycle.
    stride_m: f64,
    sprinting: bool,
    /// In water too deep to stand in. Only ever true in [`Mode::Walk`].
    swimming: bool,
    site: Site,
    moon: Option<MoonBody>,
    /// The footprint the ground around the avatar is drawn with right now:
    /// see [`Controller::shown_ground_m`].
    drawn_footprint_m: f64,
    /// Up of the avatar's own frame: what the camera, the controls and the
    /// body stand on. It turns toward gravity by rotation, never by a cut:
    /// quickly on foot, and in flight only near a surface, because in flight
    /// the flyer's frame wins. Look at the planet from the moon and fly: you go
    /// to the planet, and nothing turns you on the way.
    frame_up: DVec3,
}

impl Controller {
    /// Stands on the ground at `point`, looking along the sector's `u` axis.
    pub fn spawn(point: SurfacePoint, generator: &Generator) -> Controller {
        let sphere = generator.sphere();
        let tangents = sphere.blocks().tangents(point);
        let view = DVec3::from(tangents.du).normalize();
        Controller {
            mode: Mode::Walk,
            sphere,
            point,
            height_m: generator.sample(sphere.blocks().direction(point)).height_m,
            vertical_mps: 0.0,
            grounded: true,
            facing: view,
            view,
            pitch: -0.25,
            boom_m: 6.0,
            speed_mps: 0.0,
            ground_mps: 0.0,
            stride_m: 0.0,
            sprinting: false,
            swimming: false,
            site: Site::Planet,
            moon: None,
            drawn_footprint_m: 0.0,
            frame_up: DVec3::from(tangents.up),
        }
    }

    /// The body this controller is on.
    pub fn sphere(&self) -> QuadSphere {
        self.sphere
    }

    /// The unit direction from the body's centre through the avatar.
    fn point_direction(&self) -> [f64; 3] {
        self.sphere.blocks().direction(self.point)
    }

    pub fn point(&self) -> SurfacePoint {
        self.point
    }

    /// Up of the avatar's frame. See the `frame_up` field.
    pub fn up(&self) -> DVec3 {
        self.frame_up
    }

    /// From the centre of the site's body through the avatar.
    pub fn radial(&self) -> DVec3 {
        match self.site {
            Site::Planet => DVec3::from(self.point_direction()),
            Site::Moon { direction } => direction,
        }
    }

    /// How much of "down" belongs to the moon here, `0..=1`.
    fn moon_pull(&self) -> f64 {
        match (self.site, self.moon) {
            (Site::Moon { .. }, Some(moon)) => {
                let radii = (moon.radius_m + self.height_m) / moon.radius_m;
                1.0 - ((radii - MOON_PULL_NEAR) / (MOON_PULL_FAR - MOON_PULL_NEAR)).clamp(0.0, 1.0)
            }
            _ => 0.0,
        }
    }

    /// Against gravity, here. Between the bodies it turns continuously from
    /// one to the other: the first gravity fields (docs/ARCHITECTURE.md).
    pub fn gravity_up(&self) -> DVec3 {
        let pull = self.moon_pull();
        let planet_up = self.position().normalize();
        if pull <= 0.0 {
            return planet_up;
        }
        let eased = pull * pull * (3.0 - 2.0 * pull);
        rotate_toward(planet_up, self.radial(), eased, self.view)
    }

    /// The avatar's frame, columns right, up, back.
    pub fn body_basis(&self) -> DMat3 {
        crate::figure::basis(self.frame_up, self.facing)
    }

    pub fn on_moon(&self) -> bool {
        self.site != Site::Planet
    }

    /// Feet, metres above the datum sphere of whichever body holds them.
    pub fn height_m(&self) -> f64 {
        self.height_m
    }

    /// Camera pitch, radians, positive looks up.
    pub fn pitch(&self) -> f64 {
        self.pitch
    }

    /// Turns the avatar and the camera to a bearing, degrees clockwise from
    /// north. The same convention `topology::bearing_deg` reads, so a bearing
    /// written down and handed back points the same way.
    pub fn face(&mut self, bearing_deg: f64) {
        const NORTH: DVec3 = DVec3::Y;
        let up = self.up();
        let north = NORTH - up * up.dot(NORTH);
        if north.length() < 1e-6 {
            // At a pole there is no bearing to honour, so nothing turns.
            return;
        }
        let north = north.normalize();
        let east = north.cross(up);
        let radians = bearing_deg.to_radians();
        let heading = north * radians.cos() + east * radians.sin();
        self.view = heading.normalize();
        self.facing = self.view;
    }

    /// Stands the avatar at a height of its own rather than on the ground.
    /// `None` puts it back on the ground, which is what standing means.
    pub fn stand_at(&mut self, height_m: Option<f64>, generator: &Generator) {
        let ground = self.ground_m(generator);
        match height_m {
            None => {
                self.height_m = ground;
                self.grounded = true;
                self.mode = Mode::Walk;
            }
            Some(wanted) => {
                self.height_m = wanted.max(ground);
                self.grounded = self.height_m <= ground + 1e-6;
                if !self.grounded {
                    self.mode = Mode::Fly;
                }
            }
        }
        self.vertical_mps = 0.0;
    }

    /// Puts the avatar on the moon, at a direction from its centre. The moon
    /// has to be where the clock says before this, because a body without one
    /// has nothing to stand on.
    pub fn stand_on_moon(&mut self, direction: DVec3, generator: &Generator) {
        self.site = Site::Moon {
            direction: direction.normalize_or(DVec3::Y),
        };
        self.height_m = self.ground_m(generator);
        self.grounded = true;
        self.mode = Mode::Walk;
        self.vertical_mps = 0.0;
        let tangents = self.sphere.blocks().tangents(self.point);
        self.view = DVec3::from(tangents.du).normalize();
        self.facing = self.view;
    }

    /// Centre and datum radius of the body that holds the avatar.
    fn body(&self) -> (DVec3, f64) {
        match (self.site, self.moon) {
            (Site::Moon { .. }, Some(moon)) => (moon.center, moon.radius_m),
            _ => (DVec3::ZERO, self.sphere.radius_m()),
        }
    }

    /// Ground height over the datum of the current body, metres.
    fn ground_m(&self, generator: &Generator) -> f64 {
        match self.site {
            Site::Planet => generator.sample(self.point_direction()).height_m,
            Site::Moon { direction } => {
                generator.moon_sample_at(direction.to_array(), 0.0).height_m
            }
        }
    }

    /// What the body at this direction stands on, and what is over its head.
    /// Out in the open the floor is the surface and there is no roof; in a
    /// cave both are the cave's own, which is the whole reason the ground
    /// under the feet is a volume and not a height.
    fn footing(&self, generator: &Generator) -> Footing {
        match self.site {
            Site::Planet => collision::footing(generator, self.point_direction(), self.height_m),
            // The moon is a height all the way down, and so is any world on a
            // generator version frozen before caves.
            Site::Moon { direction } => {
                Footing::solid(generator.moon_sample_at(direction.to_array(), 0.0).height_m)
            }
        }
    }

    /// Whether the body is under the ground as the heightfield draws it: in a
    /// cave, or in the rock around one. Rules that keep a body over the
    /// terrain point the wrong way there.
    fn underground(&self, ground_m: f64) -> bool {
        self.site == Site::Planet && self.height_m < ground_m
    }

    /// Tells the controller how coarse the mesh around the avatar is this frame.
    pub fn set_drawn_footprint(&mut self, footprint_m: f64) {
        self.drawn_footprint_m = footprint_m;
    }

    /// The ground as it is on screen along `direction` from the body's centre:
    /// the real ground, or the coarser mesh over it while finer patches are
    /// still streaming in, whichever is higher. Physics on foot uses the real
    /// ground; the camera and a flyer stay above this one, so nobody ends up
    /// looking at the terrain from underneath.
    fn shown_ground_m(&self, generator: &Generator, direction: DVec3) -> f64 {
        let at = |footprint_m: f64| match self.site {
            Site::Planet => {
                generator
                    .sample_at(direction.to_array(), footprint_m)
                    .height_m
            }
            Site::Moon { .. } => {
                generator
                    .moon_sample_at(direction.to_array(), footprint_m)
                    .height_m
            }
        };
        at(0.0).max(at(self.drawn_footprint_m))
    }

    fn gravity_mps2(&self) -> f64 {
        let pull = self.moon_pull();
        GRAVITY_MPS2 + (MOON_GRAVITY_MPS2 - GRAVITY_MPS2) * pull
    }

    /// Feet in world space.
    pub fn position(&self) -> DVec3 {
        let (center, radius_m) = self.body();
        center + self.radial() * (radius_m + self.height_m)
    }

    /// Where the camera looks, flattened on the tangent plane.
    ///
    /// This, and not [`Controller::facing`], is what a compass reads: standing
    /// still and turning the mouse moves the view and leaves the body where it
    /// was, and "which way am I looking" is a question about the eyes.
    pub fn view(&self) -> DVec3 {
        self.view
    }

    pub fn facing(&self) -> DVec3 {
        self.facing
    }

    pub fn speed_mps(&self) -> f64 {
        self.speed_mps
    }

    pub fn ground_mps(&self) -> f64 {
        self.ground_mps
    }

    /// Metres per second along up: positive while rising.
    pub fn vertical_mps(&self) -> f64 {
        self.vertical_mps
    }

    /// Tells the controller where the moon is this frame.
    pub fn set_moon(&mut self, moon: MoonBody) {
        self.moon = Some(moon);
    }

    /// Metres of free space around the avatar: to the ground or sea below, or
    /// to the moon, whichever is nearer. Flight speed scales with it, so a
    /// trip to the moon is quick and the arrival is gentle.
    fn clearance_m(&self, generator: &Generator) -> f64 {
        let to_moon = match (self.site, self.moon) {
            (Site::Planet, Some(moon)) => (self.position() - moon.center).length() - moon.radius_m,
            _ => f64::MAX,
        };
        self.altitude_m(generator).min(to_moon).max(0.0)
    }

    /// Hands the avatar to the frame of reference it is in now. World position
    /// does not change, only what it is measured from.
    fn resolve_site(&mut self) {
        let Some(moon) = self.moon else {
            return;
        };
        let position = self.position();
        let offset = position - moon.center;
        let radii = offset.length() / moon.radius_m;
        match self.site {
            Site::Planet if radii < MOON_SOI_ENTER => {
                self.site = Site::Moon {
                    direction: offset.normalize_or(DVec3::Y),
                };
                self.height_m = offset.length() - moon.radius_m;
            }
            Site::Moon { .. } if radii > MOON_SOI_EXIT => {
                self.site = Site::Planet;
                self.point = self.sphere.blocks().surface_point(position.to_array());
                self.height_m = position.length() - self.sphere.radius_m();
            }
            _ => {}
        }
    }

    /// Turns the avatar's frame toward gravity: see the `frame_up` field.
    fn align_frame(&mut self, dt: f64, generator: &Generator) {
        let rate = match self.mode {
            Mode::Walk => WALK_ALIGN_PER_S,
            Mode::Fly => {
                let body_radius = self.body().1;
                let clearance = self.altitude_m(generator).max(0.0);
                let far =
                    (clearance / body_radius - FLY_ALIGN_NEAR) / (FLY_ALIGN_FAR - FLY_ALIGN_NEAR);
                FLY_ALIGN_PER_S * (1.0 - far.clamp(0.0, 1.0))
            }
        };
        // The whole frame turns as one: what the avatar looks at and faces
        // turns with its up, so aligning never reads as a camera move.
        let fraction = 1.0 - (-rate * dt).exp();
        let turn = turn_toward(self.frame_up, self.gravity_up(), fraction, self.view);
        self.frame_up = (turn * self.frame_up).normalize();
        self.view = turn * self.view;
        self.facing = turn * self.facing;
    }

    pub fn swimming(&self) -> bool {
        self.swimming
    }

    /// Swimming under the surface.
    pub fn diving(&self) -> bool {
        self.swimming && self.height_m < DIVING_M
    }

    /// Sprint is held while moving.
    pub fn sprinting(&self) -> bool {
        self.sprinting
    }

    pub fn stride_m(&self) -> f64 {
        self.stride_m
    }

    pub fn grounded(&self) -> bool {
        self.grounded
    }

    /// Flies to a place in world space.
    pub fn fly_to(&mut self, position: DVec3) {
        self.site = Site::Planet;
        self.point = self.sphere.blocks().surface_point(position.to_array());
        self.height_m = position.length() - self.sphere.radius_m();
        self.mode = Mode::Fly;
        self.grounded = false;
        self.resolve_site();
        self.frame_up = self.gravity_up();
        self.transport();
    }

    /// Moves to a point of the surface, standing on the ground there.
    pub fn teleport(&mut self, point: SurfacePoint, generator: &Generator) {
        *self = Controller {
            mode: self.mode,
            pitch: self.pitch,
            boom_m: self.boom_m,
            ..Controller::spawn(point, generator)
        };
    }

    /// Aims the camera up or down, radians, positive looks up. The boom is
    /// left where it was: how far back someone watches from is theirs.
    pub fn set_pitch(&mut self, pitch: f64) {
        self.pitch = pitch.clamp(-PITCH_LIMIT, PITCH_LIMIT);
    }

    /// Aims the preview camera: `pitch` in radians, `boom_m` behind the avatar.
    pub fn pose_view(&mut self, pitch: f64, boom_m: f64) {
        self.pitch = pitch.clamp(-PITCH_LIMIT, PITCH_LIMIT);
        self.boom_m = boom_m;
    }

    /// Places the avatar and the camera for a preview shot: `altitude_m` above
    /// the ground (flying when positive), camera `pitch` in radians, `boom_m`
    /// behind the avatar.
    pub fn pose(&mut self, altitude_m: f64, pitch: f64, boom_m: f64, generator: &Generator) {
        let ground = self.ground_m(generator);
        // Negative altitudes are depths under the sea, never under the ground.
        self.height_m = (ground.max(0.0) + altitude_m).max(ground);
        self.mode = if altitude_m > 0.0 {
            Mode::Fly
        } else {
            Mode::Walk
        };
        self.grounded = altitude_m <= 0.0;
        self.pose_view(pitch, boom_m);
    }

    /// Metres above the ground, or above the sea where there is one. Negative
    /// under water.
    pub fn altitude_m(&self, generator: &Generator) -> f64 {
        let ground = self.ground_m(generator);
        // Only the planet has a sea to be above.
        let surface = if self.site == Site::Planet {
            ground.max(0.0)
        } else {
            ground
        };
        self.height_m - surface
    }

    pub fn update(&mut self, dt: f64, wish: Wish, generator: &Generator) {
        // `up` is the avatar's own frame: look, steer and fly by it. `gravity`
        // is the world's: fall and float by it. On foot the two agree within a
        // moment; in flight they may not, and the frame wins.
        let up = self.frame_up;
        let gravity = self.gravity_up();
        self.look(wish, up);

        let right = self.view.cross(up);
        let [x, y] = wish.movement.map(f64::from);
        let flat = (right * x + self.view * y).normalize_or_zero();
        let sprint = match (wish.sprint, self.mode) {
            (false, _) => 1.0,
            (true, Mode::Walk) => RUN_FACTOR,
            (true, Mode::Fly) => FLY_SPRINT_FACTOR,
        };

        // Deep enough to float, and sunk to floating depth: swim. Wading in
        // from a beach and falling in from a cliff both end up here.
        let ground = self.ground_m(generator);
        self.swimming = self.site == Site::Planet
            && self.mode == Mode::Walk
            && ground < FLOAT_M - 0.1
            && self.height_m <= FLOAT_M + 0.05;

        let lift = f64::from(u8::from(wish.up)) - f64::from(u8::from(wish.down));
        let velocity = match self.mode {
            // A leap out of the water: from the surface only. Gravity brings
            // the swimmer back, and they are swimming again on the way down.
            Mode::Walk if self.swimming && wish.up && !self.diving() => {
                self.swimming = false;
                self.vertical_mps = JUMP_MPS * 0.8;
                self.height_m = FLOAT_M + 0.06;
                flat * SWIM_MPS + gravity * self.vertical_mps
            }
            Mode::Walk if self.swimming => {
                self.vertical_mps = 0.0;
                self.grounded = false;
                // Under water the camera steers. At the surface you swim flat,
                // unless you look well down: then forward is a dive.
                let steered = self.diving() || self.pitch < DIVE_PITCH;
                let forward = if steered {
                    self.view * self.pitch.cos() + up * self.pitch.sin()
                } else {
                    self.view
                };
                let wish_dir = (right * x + forward * y + gravity * lift).normalize_or_zero();
                let pace = if wish.sprint { SWIM_SPRINT_FACTOR } else { 1.0 };
                let drift = if wish_dir == DVec3::ZERO {
                    gravity * BUOYANCY_MPS
                } else {
                    DVec3::ZERO
                };
                wish_dir * SWIM_MPS * pace + drift
            }
            Mode::Walk => {
                if self.grounded && wish.up {
                    self.vertical_mps = JUMP_MPS;
                    self.grounded = false;
                }
                if !self.grounded {
                    self.vertical_mps -= self.gravity_mps2() * dt;
                }
                flat * WALK_MPS * sprint + gravity * self.vertical_mps
            }
            Mode::Fly => {
                self.vertical_mps = 0.0;
                self.grounded = false;
                // Forward follows the camera pitch: look up, fly up.
                let forward = self.view * self.pitch.cos() + up * self.pitch.sin();
                let wish_dir = (right * x + forward * y + up * lift).normalize_or_zero();
                let clearance = self.clearance_m(generator);
                wish_dir * (FLY_MIN_MPS + clearance * FLY_PER_ALTITUDE) * sprint
            }
        };

        self.step(velocity * dt, generator);
        self.resolve_site();
        self.speed_mps = velocity.length();
        self.sprinting = wish.sprint && self.speed_mps > 0.1;
        self.ground_mps = (velocity - gravity * velocity.dot(gravity)).length();
        if self.grounded {
            self.stride_m += flat.length() * WALK_MPS * sprint * dt;
        }

        // Turn the body toward where it goes; in flight, where the camera looks.
        let target = match self.mode {
            Mode::Walk => flat,
            Mode::Fly => self.view,
        };
        if target != DVec3::ZERO {
            let blend = 1.0 - (-TURN_PER_S * dt).exp();
            self.facing = self.facing.lerp(target, blend);
        }
        self.align_frame(dt, generator);
        self.transport();
    }

    /// Applies pointer look and zoom.
    fn look(&mut self, wish: Wish, up: DVec3) {
        let yaw = -f64::from(wish.look[0]) * LOOK_RAD_PER_PX;
        self.view = DQuat::from_axis_angle(up, yaw) * self.view;
        self.pitch = (self.pitch - f64::from(wish.look[1]) * LOOK_RAD_PER_PX)
            .clamp(-PITCH_LIMIT, PITCH_LIMIT);
        self.boom_m = (self.boom_m * (1.0 - f64::from(wish.zoom) * 0.1)).clamp(2.0, 400.0);
    }

    /// Moves by a world space displacement: in address space on the planet,
    /// over a plain sphere on the moon.
    fn step(&mut self, delta: DVec3, generator: &Generator) {
        match self.site {
            Site::Planet => self.step_on_planet(delta, generator),
            Site::Moon { direction } => {
                let radius = self.body().1 + self.height_m;
                let along = delta - direction * direction.dot(delta);
                self.site = Site::Moon {
                    direction: (direction * radius + along).normalize(),
                };
                self.height_m += direction.dot(delta);
            }
        }

        let ground = self.ground_m(generator);
        if self.swimming {
            // Between the sea floor and floating depth: nobody swims into the air.
            self.height_m = self.height_m.min(FLOAT_M).max(ground);
            return;
        }
        let footing = self.footing(generator);
        let floor_m = match self.mode {
            // A flyer is held over the ground as it is drawn, so it never
            // ends up looking at the terrain from underneath. Under the
            // ground that rule points the wrong way, and the footing, which
            // knows about the cave, takes over.
            Mode::Fly if !self.underground(ground) => {
                Some(self.shown_ground_m(generator, self.radial()) + 0.5)
            }
            _ => footing.floor_m,
        };
        match floor_m {
            // Nothing within reach under the feet: keep falling.
            None => self.grounded = false,
            Some(floor_m) if self.height_m <= floor_m => {
                self.height_m = floor_m;
                if self.mode == Mode::Walk {
                    self.grounded = true;
                    self.vertical_mps = 0.0;
                }
            }
            Some(floor_m) if self.mode == Mode::Walk && self.grounded => {
                // Walking downhill: stay glued to the ground over a step
                // (auto step), fall off anything deeper.
                if self.height_m - floor_m < STEP_M {
                    self.height_m = floor_m;
                } else {
                    self.grounded = false;
                }
            }
            Some(_) => {}
        }

        // A roof stops a rise the way the ground stops a fall. Squeezed into
        // a gap thinner than a body, the floor wins: better low than sunk.
        if let Some(ceiling_m) = footing.ceiling_m {
            let head_m = (ceiling_m - BODY_M).max(floor_m.unwrap_or(f64::MIN));
            if self.height_m > head_m {
                self.height_m = head_m;
                self.vertical_mps = self.vertical_mps.min(0.0);
            }
        }
    }

    fn step_on_planet(&mut self, delta: DVec3, generator: &Generator) {
        let tangents = self.sphere.blocks().tangents(self.point);
        let up = DVec3::from(tangents.up);
        let radius = self.sphere.radius_m() + self.height_m;
        // Metres per block along each address axis, at this height.
        let (tu, tv) = (
            DVec3::from(tangents.du) * radius,
            DVec3::from(tangents.dv) * radius,
        );

        // Solve `du * tu + dv * tv = delta` on the tangent plane. The axes are
        // not orthogonal, so this is a 2x2 system, not two dot products.
        let (a, b, c) = (tu.dot(tu), tu.dot(tv), tv.dot(tv));
        let (p, q) = (tu.dot(delta), tv.dot(delta));
        let det = a * c - b * b;
        let (du, dv) = ((c * p - b * q) / det, (a * q - b * p) / det);

        self.point = if self.stopped_by_rock(generator) {
            self.walk_to(du, dv, generator)
        } else {
            self.sphere.blocks().wrapped(SurfacePoint::new(
                self.point.sector,
                self.point.u + du,
                self.point.v + dv,
            ))
        };
        self.height_m += up.dot(delta);
    }

    /// Whether rock in the way stops this body. A walker, always; a flyer
    /// only inside the ground, where it is in a cave and the walls are real.
    /// Swimmers pass: the sea has no walls, and its floor is a height.
    fn stopped_by_rock(&self, generator: &Generator) -> bool {
        !self.swimming && (self.mode == Mode::Walk || self.underground(self.ground_m(generator)))
    }

    /// Where a step of `du, dv` in address space ends: where it asked, or
    /// short of whatever stopped it.
    ///
    /// A rise of one block is taken in stride and more is a wall, so a cave
    /// wall stops a body exactly the way a cliff does and the only way past
    /// either is to jump or to fly. Blocked, the step is tried one address
    /// axis at a time, which is what slides a body along a wall instead of
    /// sticking it to one.
    fn walk_to(&self, du: f64, dv: f64, generator: &Generator) -> SurfacePoint {
        let here = self.footing(generator);
        for (du, dv) in [(du, dv), (du, 0.0), (0.0, dv)] {
            if du == 0.0 && dv == 0.0 {
                continue;
            }
            let point = self.sphere.blocks().wrapped(SurfacePoint::new(
                self.point.sector,
                self.point.u + du,
                self.point.v + dv,
            ));
            let there = collision::footing(
                generator,
                self.sphere.blocks().direction(point),
                self.height_m,
            );
            if collision::admits(here, there, self.height_m) {
                return point;
            }
        }
        self.point
    }

    /// Keeps the tangent vectors tangent to the frame: projected back on the
    /// plane under `frame_up`. With the frame following gravity as the avatar
    /// moves this is parallel transport, and it is why seams and corners need
    /// no special case.
    fn transport(&mut self) {
        let up = self.frame_up;
        let flatten = |v: DVec3, fallback: DVec3| {
            let flat = (v - up * v.dot(up)).normalize_or_zero();
            if flat == DVec3::ZERO { fallback } else { flat }
        };
        let any = up.any_orthonormal_vector();
        self.view = flatten(self.view, any);
        self.facing = flatten(self.facing, self.view);
    }

    /// The third person camera: behind and above the avatar, never underground.
    pub fn camera(&self, generator: &Generator) -> Camera {
        let up = self.frame_up;
        let forward = self.view * self.pitch.cos() + up * self.pitch.sin();
        let target = self.position() + up * EYE_M;
        let mut position = target - forward * self.boom_m;

        // Never inside the body the avatar stands on, as drawn. In a cave
        // that rule would yank the camera out through the roof, so there the
        // boom is cut by the rock behind it instead: the camera stays in the
        // cave with the body, as near to it as the walls allow.
        let (center, datum_m) = self.body();
        if self.underground(self.ground_m(generator)) {
            let run_m = collision::clear_run_m(generator, target, -forward, self.boom_m);
            position = target - forward * run_m;
        } else {
            let from_center = position - center;
            let ground = self.shown_ground_m(generator, from_center.normalize());
            let min_radius = datum_m + ground + 0.4;
            if from_center.length() < min_radius {
                position = center + from_center.normalize() * min_radius;
            }
        }

        // Aim at the target from wherever the camera ended up.
        let look = (target - position).normalize();
        let right = look.cross(up).normalize();
        let rotation = DQuat::from_mat3(&DMat3::from_cols(right, right.cross(look), -look));
        Camera {
            position,
            rotation,
            fov_y: 60f32.to_radians(),
            near: 0.1,
        }
    }
}

/// The rotation that takes `from` a `fraction` of the way to `to`, both unit.
/// When they are opposite any axis would do, so the caller names one.
fn turn_toward(from: DVec3, to: DVec3, fraction: f64, fallback_axis: DVec3) -> DQuat {
    let angle = from.dot(to).clamp(-1.0, 1.0).acos();
    if angle < 1e-9 {
        return DQuat::IDENTITY;
    }
    let axis = from.cross(to).normalize_or(
        fallback_axis
            .cross(from)
            .normalize_or(from.any_orthonormal_vector()),
    );
    DQuat::from_axis_angle(axis, angle * fraction)
}

/// `from` turned a `fraction` of the way to `to`.
fn rotate_toward(from: DVec3, to: DVec3, fraction: f64, fallback_axis: DVec3) -> DVec3 {
    (turn_toward(from, to, fraction, fallback_axis) * from).normalize()
}
