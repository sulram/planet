//! The avatar controller: walk with radial gravity, swim, fly like superman.
//!
//! State lives in address space (`SurfacePoint` + height), the one place where
//! blocks are unit cubes and where collision with the build layer will run.
//! World space enters only as velocities: a wish direction in metres is
//! converted to an address space delta through the local tangents, so speed
//! feels the same everywhere although blocks are not perfectly square.

use glam::{DMat3, DQuat, DVec3};
use scene::Camera;
use topology::{RADIUS_M, SurfacePoint};
use worldgen::Generator;

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
/// Reach of the moon's gravity field above its surface, as a share of its
/// radius. Inside it the moon is down. Leaving takes a little more height than
/// entering, so the edge never flickers.
const MOON_FIELD: f64 = 0.75;
const MOON_FIELD_EXIT: f64 = 0.9;
/// How fast the body and camera swing to a new up, per second.
const UP_EASE_PER_S: f64 = 2.5;
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

/// The celestial body whose gravity field holds the avatar. It decides which
/// way is down and what the position is stored relative to, so an avatar on
/// the moon rides along its orbit without anyone moving it.
///
/// This is the first gravity field (docs/ARCHITECTURE.md, Gravity): a sphere
/// with a range. The highest priority field containing you wins; with two
/// bodies that is "the moon's, when inside it".
#[derive(Clone, Copy, PartialEq, Debug)]
enum Site {
    /// Position is `point` + `height_m` over the datum sphere.
    Planet,
    /// Position is `direction` (unit, from the moon's centre) + `height_m`
    /// over the moon's surface. The moon is a smooth ball for now.
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
    /// Up as the body and the camera show it: eased toward the true up, so
    /// entering another gravity field swings the world round instead of
    /// snapping it.
    body_up: DVec3,
}

impl Controller {
    /// Stands on the ground at `point`, looking along the sector's `u` axis.
    pub fn spawn(point: SurfacePoint, generator: &Generator) -> Controller {
        let tangents = point.tangents();
        let view = DVec3::from(tangents.du).normalize();
        Controller {
            mode: Mode::Walk,
            point,
            height_m: generator.sample(point.direction()).height_m,
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
            body_up: DVec3::from(tangents.up),
        }
    }

    pub fn point(&self) -> SurfacePoint {
        self.point
    }

    /// True up: against the gravity of the body that holds the avatar.
    pub fn up(&self) -> DVec3 {
        match self.site {
            Site::Planet => DVec3::from(self.point.direction()),
            Site::Moon { direction } => direction,
        }
    }

    /// Up as shown: see [`Controller::body_basis`].
    pub fn body_up(&self) -> DVec3 {
        self.body_up
    }

    /// The avatar's frame as shown, columns right, up, back: eased up, and
    /// the facing flattened against it.
    pub fn body_basis(&self) -> DMat3 {
        let up = self.body_up;
        let back =
            -(self.facing - up * self.facing.dot(up)).normalize_or(up.any_orthonormal_vector());
        DMat3::from_cols(up.cross(back), up, back)
    }

    pub fn on_moon(&self) -> bool {
        self.site != Site::Planet
    }

    /// Centre and datum radius of the body that holds the avatar.
    fn body(&self) -> (DVec3, f64) {
        match (self.site, self.moon) {
            (Site::Moon { .. }, Some(moon)) => (moon.center, moon.radius_m),
            _ => (DVec3::ZERO, RADIUS_M),
        }
    }

    /// Ground height over the datum of the current body, metres.
    fn ground_m(&self, generator: &Generator) -> f64 {
        match self.site {
            Site::Planet => generator.sample(self.point.direction()).height_m,
            Site::Moon { .. } => 0.0,
        }
    }

    fn gravity_mps2(&self) -> f64 {
        match self.site {
            Site::Planet => GRAVITY_MPS2,
            Site::Moon { .. } => MOON_GRAVITY_MPS2,
        }
    }

    /// Feet in world space.
    pub fn position(&self) -> DVec3 {
        let (center, radius_m) = self.body();
        center + self.up() * (radius_m + self.height_m)
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

    /// Hands the avatar to whichever gravity field it is in now.
    fn resolve_site(&mut self) {
        let Some(moon) = self.moon else {
            return;
        };
        let position = self.position();
        match self.site {
            Site::Planet => {
                let offset = position - moon.center;
                if offset.length() < moon.radius_m * (1.0 + MOON_FIELD) {
                    self.site = Site::Moon {
                        direction: offset.normalize_or(DVec3::Y),
                    };
                    self.height_m = offset.length() - moon.radius_m;
                    self.vertical_mps = 0.0;
                }
            }
            Site::Moon { .. } => {
                if self.height_m > moon.radius_m * MOON_FIELD_EXIT {
                    self.site = Site::Planet;
                    self.point = SurfacePoint::from_direction(position.to_array());
                    self.height_m = position.length() - RADIUS_M;
                    self.vertical_mps = 0.0;
                }
            }
        }
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
        self.point = SurfacePoint::from_direction(position.to_array());
        self.height_m = position.length() - RADIUS_M;
        self.mode = Mode::Fly;
        self.grounded = false;
        self.resolve_site();
        self.body_up = self.up();
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

    /// Metres above the ground or the sea. Negative under water.
    pub fn altitude_m(&self, generator: &Generator) -> f64 {
        self.height_m - self.ground_m(generator).max(0.0)
    }

    pub fn update(&mut self, dt: f64, wish: Wish, generator: &Generator) {
        let up = self.up();
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
                flat * SWIM_MPS + up * self.vertical_mps
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
                let wish_dir = (right * x + forward * y + up * lift).normalize_or_zero();
                let pace = if wish.sprint { SWIM_SPRINT_FACTOR } else { 1.0 };
                let drift = if wish_dir == DVec3::ZERO {
                    up * BUOYANCY_MPS
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
                flat * WALK_MPS * sprint + up * self.vertical_mps
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
        self.ground_mps = (velocity - up * velocity.dot(up)).length();
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
        self.transport();
        let ease = 1.0 - (-UP_EASE_PER_S * dt).exp();
        self.body_up = self.body_up.lerp(self.up(), ease).normalize_or(self.up());
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
            Site::Planet => self.step_on_planet(delta),
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
        let floor = match self.mode {
            Mode::Walk => ground,
            Mode::Fly => ground + 0.5,
        };
        if self.height_m <= floor {
            self.height_m = floor;
            if self.mode == Mode::Walk {
                self.grounded = true;
                self.vertical_mps = 0.0;
            }
        } else if self.mode == Mode::Walk && self.grounded {
            // Walking downhill: stay glued to the ground over small drops
            // (auto step), fall off real ledges.
            if self.height_m - ground < 0.6 {
                self.height_m = ground;
            } else {
                self.grounded = false;
            }
        }
    }

    fn step_on_planet(&mut self, delta: DVec3) {
        let tangents = self.point.tangents();
        let up = DVec3::from(tangents.up);
        let radius = RADIUS_M + self.height_m;
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

        self.point =
            SurfacePoint::new(self.point.sector, self.point.u + du, self.point.v + dv).wrapped();
        self.height_m += up.dot(delta);
    }

    /// Keeps the tangent vectors tangent after the avatar moved: up changed,
    /// so they are projected back on the new tangent plane. This is parallel
    /// transport, and it is why seams and corners need no special case.
    fn transport(&mut self) {
        let up = self.up();
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
        // The shown up, and the view flattened against it: while a new gravity
        // field swings the body round, the camera swings with it.
        let up = self.body_up;
        let view = (self.view - up * self.view.dot(up)).normalize_or(self.view);
        let forward = view * self.pitch.cos() + up * self.pitch.sin();
        let target = self.position() + up * EYE_M;
        let mut position = target - forward * self.boom_m;

        // Never inside the body the avatar stands on.
        let (center, datum_m) = self.body();
        let from_center = position - center;
        let ground = match self.site {
            Site::Planet => {
                generator
                    .sample(from_center.normalize().to_array())
                    .height_m
            }
            Site::Moon { .. } => 0.0,
        };
        let min_radius = datum_m + ground + 0.4;
        if from_center.length() < min_radius {
            position = center + from_center.normalize() * min_radius;
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
