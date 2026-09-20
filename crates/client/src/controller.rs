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
        }
    }

    pub fn point(&self) -> SurfacePoint {
        self.point
    }

    pub fn up(&self) -> DVec3 {
        DVec3::from(self.point.direction())
    }

    /// Feet in world space.
    pub fn position(&self) -> DVec3 {
        self.up() * (RADIUS_M + self.height_m)
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

    /// Moves to a point of the surface, standing on the ground there.
    pub fn teleport(&mut self, point: SurfacePoint, generator: &Generator) {
        *self = Controller {
            mode: self.mode,
            pitch: self.pitch,
            boom_m: self.boom_m,
            ..Controller::spawn(point, generator)
        };
    }

    /// Places the avatar and the camera for a preview shot: `altitude_m` above
    /// the ground (flying when positive), camera `pitch` in radians, `boom_m`
    /// behind the avatar.
    pub fn pose(&mut self, altitude_m: f64, pitch: f64, boom_m: f64, generator: &Generator) {
        let ground = generator.sample(self.point.direction()).height_m;
        // Negative altitudes are depths under the sea, never under the ground.
        self.height_m = (ground.max(0.0) + altitude_m).max(ground);
        self.mode = if altitude_m > 0.0 {
            Mode::Fly
        } else {
            Mode::Walk
        };
        self.grounded = altitude_m <= 0.0;
        self.pitch = pitch.clamp(-PITCH_LIMIT, PITCH_LIMIT);
        self.boom_m = boom_m;
    }

    /// Metres above the ground or the sea. Negative under water.
    pub fn altitude_m(&self, generator: &Generator) -> f64 {
        self.height_m - generator.sample(self.point.direction()).height_m.max(0.0)
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
        let ground = generator.sample(self.point.direction()).height_m;
        self.swimming =
            self.mode == Mode::Walk && ground < FLOAT_M - 0.1 && self.height_m <= FLOAT_M + 0.05;

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
                    self.vertical_mps -= GRAVITY_MPS2 * dt;
                }
                flat * WALK_MPS * sprint + up * self.vertical_mps
            }
            Mode::Fly => {
                self.vertical_mps = 0.0;
                self.grounded = false;
                // Forward follows the camera pitch: look up, fly up.
                let forward = self.view * self.pitch.cos() + up * self.pitch.sin();
                let wish_dir = (right * x + forward * y + up * lift).normalize_or_zero();
                let altitude = self.altitude_m(generator).max(0.0);
                wish_dir * (FLY_MIN_MPS + altitude * FLY_PER_ALTITUDE) * sprint
            }
        };

        self.step(velocity * dt, generator);
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
    }

    /// Applies pointer look and zoom.
    fn look(&mut self, wish: Wish, up: DVec3) {
        let yaw = -f64::from(wish.look[0]) * LOOK_RAD_PER_PX;
        self.view = DQuat::from_axis_angle(up, yaw) * self.view;
        self.pitch = (self.pitch - f64::from(wish.look[1]) * LOOK_RAD_PER_PX)
            .clamp(-PITCH_LIMIT, PITCH_LIMIT);
        self.boom_m = (self.boom_m * (1.0 - f64::from(wish.zoom) * 0.1)).clamp(2.0, 400.0);
    }

    /// Moves by a world space displacement, resolved in address space.
    fn step(&mut self, delta: DVec3, generator: &Generator) {
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

        let ground = generator.sample(self.point.direction()).height_m;
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
        let up = self.up();
        let forward = self.view * self.pitch.cos() + up * self.pitch.sin();
        let target = self.position() + up * EYE_M;
        let mut position = target - forward * self.boom_m;

        let camera_up = position.normalize();
        let ground = generator.sample(camera_up.to_array()).height_m;
        let min_radius = RADIUS_M + ground + 0.4;
        if position.length() < min_radius {
            position = camera_up * min_radius;
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
