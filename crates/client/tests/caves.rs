//! A cave is ground: you stand on its floor and not on its roof.
//!
//! Every place here is searched for rather than written down, so the tests
//! still mean something the day the generator moves a cave.

use client::collision::{self, BODY_M, STEP_M};
use client::{Controller, Wish};
use topology::{SECTOR_SIDE, Sector, SurfacePoint};
use worldgen::{Column, Generator, Recipe};

/// Cave country of seed and place (docs/ROADMAP.md): where the renders that
/// found the caves were taken.
const CAVE_SEED: u64 = 0x0000_0000_cafe_0007;
const FROM: [f64; 2] = [0.30, 0.50];
const SPAN: f64 = 0.22;
const STEPS: i32 = 400;

fn world() -> Generator {
    Generator::new(Recipe::new(CAVE_SEED)).expect("the current generator version")
}

fn at(u: f64, v: f64) -> SurfacePoint {
    let side = f64::from(SECTOR_SIDE);
    SurfacePoint::new(Sector::ALL[0], u * side, v * side)
}

/// The first place of cave country whose column `wanted` accepts.
fn search(generator: &Generator, what: &str, wanted: impl Fn(&Column) -> bool) -> SurfacePoint {
    for i in 0..STEPS {
        for j in 0..STEPS {
            let step = SPAN / f64::from(STEPS);
            let point = at(FROM[0] + f64::from(i) * step, FROM[1] + f64::from(j) * step);
            let column = generator.column(point.direction(), 0.0);
            if !column.solid() && wanted(&column) {
                return point;
            }
        }
    }
    panic!("no {what} in the cave country of seed {CAVE_SEED:016x}");
}

/// The roof and floor of the first hollow under the ground of a column.
fn hollow(column: &Column) -> Option<(f64, f64)> {
    let ground_m = column.ground().height_m;
    let mut roof_m = None;
    let mut height_m = ground_m;
    while height_m > ground_m - 80.0 {
        let air = column.density_m(height_m) < 0.0;
        match (air, roof_m) {
            (true, None) if height_m < ground_m => roof_m = Some(height_m),
            (false, Some(roof_m)) => return Some((roof_m, height_m)),
            _ => {}
        }
        height_m -= 0.25;
    }
    None
}

#[test]
fn ground_without_a_cave_is_its_height() {
    let generator = world();
    let mut checked = 0;
    for i in 0..60 {
        for j in 0..60 {
            let point = at(f64::from(i) / 60.0, f64::from(j) / 60.0);
            let column = generator.column(point.direction(), 0.0);
            if !column.solid() {
                continue;
            }
            let ground_m = column.ground().height_m;
            let footing = collision::footing(&generator, point.direction(), ground_m);
            // The fast path: nothing hollow anywhere, so nothing to search.
            assert_eq!(footing.floor_m, Some(ground_m));
            assert_eq!(footing.ceiling_m, None);
            assert_eq!(footing.room_m(), f64::INFINITY);
            checked += 1;
        }
    }
    assert!(checked > 1000, "only {checked} solid columns to check");
}

#[test]
fn a_cave_is_a_floor_under_a_roof() {
    let generator = world();
    let point = search(&generator, "chamber", |column| {
        hollow(column).is_some_and(|(roof_m, floor_m)| roof_m - floor_m > BODY_M + 1.0)
    });
    let column = generator.column(point.direction(), 0.0);
    let (roof_m, floor_m) = hollow(&column).expect("the chamber the search found");
    println!(
        "chamber {floor_m:.1} m to {roof_m:.1} m, ground {:.1} m",
        column.ground().height_m
    );

    // Standing on the floor: that floor, and the roof if it is within reach.
    let standing = collision::footing(&generator, point.direction(), floor_m + 0.5);
    let found_m = standing.floor_m.expect("a floor under the feet");
    assert!(
        (found_m - floor_m).abs() < 0.3,
        "floor at {found_m}, not {floor_m}"
    );
    assert!(
        column.density_m(found_m - 0.5) > 0.0,
        "the floor is not rock"
    );
    assert!(
        column.density_m(found_m + 0.5) < 0.0,
        "the floor is not open above"
    );

    // Standing with the head almost against the roof: the roof, and the
    // room between the two.
    let head_m = roof_m - BODY_M - 0.1;
    let under = collision::footing(&generator, point.direction(), head_m);
    let ceiling_m = under.ceiling_m.expect("a roof over the feet");
    assert!(
        (ceiling_m - roof_m).abs() < 0.3,
        "roof at {ceiling_m}, not {roof_m}"
    );
    assert!(
        under.room_m() > BODY_M,
        "only {:.1} m of room",
        under.room_m()
    );
}

#[test]
fn a_mouth_is_not_a_floor() {
    let generator = world();
    // A mouth: the ground the heightfield reports is open air, because the
    // cave came out through it.
    let point = search(&generator, "cave mouth", |column| {
        let ground_m = column.ground().height_m;
        ground_m > 5.0 && column.density_m(ground_m - 0.5) < 0.0
    });
    let column = generator.column(point.direction(), 0.0);
    let ground_m = column.ground().height_m;
    let footing = collision::footing(&generator, point.direction(), ground_m);
    match footing.floor_m {
        // Deeper than a footing looks: nothing to stand on, so a body falls.
        None => {}
        Some(floor_m) => assert!(
            floor_m < ground_m - 1.0,
            "a body would stand at {floor_m} on a hole whose ground reads {ground_m}"
        ),
    }
}

/// Walks a body with `wish` for `seconds`, checking every frame that it is
/// never inside rock. Answers how many frames it spent under the ground.
fn walk(controller: &mut Controller, generator: &Generator, wish: Wish, seconds: f64) -> u32 {
    let mut underground = 0;
    for frame in 0..(seconds * 60.0) as u32 {
        controller.update(1.0 / 60.0, wish, generator);
        let column = generator.column(controller.point().direction(), 0.0);
        let feet_m = controller.position().length() - topology::RADIUS_M;
        assert!(
            column.density_m(feet_m + STEP_M) <= 0.0,
            "frame {frame}: knee deep in rock at {feet_m} m, ground {} m",
            column.ground().height_m
        );
        if feet_m < column.ground().height_m - 0.5 {
            underground += 1;
        }
    }
    underground
}

#[test]
fn a_body_falls_into_a_cave_and_stands_on_its_floor() {
    let generator = world();
    let point = search(&generator, "cave mouth", |column| {
        let ground_m = column.ground().height_m;
        ground_m > 5.0 && column.density_m(ground_m - 0.5) < 0.0
    });
    let mut controller = Controller::spawn(point, &generator);
    // Spawning stands a body on the height, which here is over a hole.
    let underground = walk(&mut controller, &generator, Wish::default(), 3.0);
    assert!(
        underground > 100,
        "only {underground} frames under the ground"
    );
    assert!(controller.grounded(), "still falling after three seconds");

    let column = generator.column(controller.point().direction(), 0.0);
    let feet_m = controller.position().length() - topology::RADIUS_M;
    let altitude_m = controller.altitude_m(&generator);
    println!("landed {altitude_m:.1} m under the height, at {feet_m:.1} m");
    assert!(altitude_m < -1.0, "landed on the roof of the hole");
    assert!(column.density_m(feet_m - 0.25) > 0.0, "standing on nothing");
}

#[test]
fn rock_stops_a_walker_from_every_direction() {
    let generator = world();
    let point = search(&generator, "cave mouth", |column| {
        let ground_m = column.ground().height_m;
        ground_m > 5.0 && column.density_m(ground_m - 0.5) < 0.0
    });
    for turn in 0..8 {
        let mut controller = Controller::spawn(point, &generator);
        walk(&mut controller, &generator, Wish::default(), 3.0);
        let angle = f64::from(turn) * core::f64::consts::TAU / 8.0;
        let wish = Wish {
            movement: [libm::sin(angle) as f32, libm::cos(angle) as f32],
            sprint: true,
            ..Wish::default()
        };
        // Long enough to cross the chamber and lean on whatever is around it.
        walk(&mut controller, &generator, wish, 6.0);
    }
}
