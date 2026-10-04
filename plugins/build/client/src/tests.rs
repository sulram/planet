//! The tool and the platform over a client's cells: the plugin held in hand
//! and given its turns one by one, with an eye placed where a test wants it.

use client::{Client, Eye, Level, Met, Recipe};
use glam::{DMat3, DQuat, DVec3};
use topology::{QuadSphere, Sector};
use voxel::{Face, Quad, Span};

use super::*;

/// Where a lattice point of a sector's cells is in the world.
fn corner(sphere: QuadSphere, sector: Sector, p: [i32; 3]) -> DVec3 {
    let point = SurfacePoint::new(sector, f64::from(p[0]), f64::from(p[1]));
    DVec3::from(sphere.position(point, f64::from(p[2]) * BLOCK_M))
}

fn world() -> (Client, SurfacePoint) {
    let mut client = Client::new(Recipe::new(1)).unwrap();
    client.rehearse(Level::Admin);
    let side = f64::from(client.sphere().blocks().side());
    let sector = Sector::new(4).unwrap();
    (client, SurfacePoint::new(sector, side * 0.41, side * 0.37))
}

fn ground(client: &mut Client, sector: Sector, u: f64, v: f64) -> f64 {
    let point = SurfacePoint::new(sector, u, v);
    client.host(NAME).ground(sector.into(), point)
}

/// Asks for a platform with the feet `height_m` over the datum and lays it
/// at once, however long the ground takes to read. The height of its top,
/// metres.
fn lay_from(
    build: &mut Build,
    client: &mut Client,
    point: SurfacePoint,
    height_m: f64,
    base: Base,
) -> Result<f64, Refusal> {
    let feet = Feet {
        seat: point.sector.into(),
        point,
        height_m,
    };
    let host = &mut client.host(NAME);
    build.lay_at(feet, base, host)?;
    let top = build
        .read_ground(host, usize::MAX)
        .expect("a platform laid");
    Ok(f64::from(top) * BLOCK_M)
}

/// The same with the feet on the ground.
fn lay_now(
    build: &mut Build,
    client: &mut Client,
    point: SurfacePoint,
    base: Base,
) -> Result<f64, Refusal> {
    let height_m = ground(client, point.sector, point.u, point.v) * BLOCK_M;
    lay_from(build, client, point, height_m, base)
}

/// The platform a body standing at a point is given, of the side picked:
/// the square the address cuts, its top where the slab is found.
fn platform_at(build: &Build, client: &Client, point: SurfacePoint) -> Platform {
    let (x, y) = (point.u.floor() as i32, point.v.floor() as i32);
    let held = client
        .cells()
        .bounds_over(point.sector.into(), point)
        .expect("an open volume");
    let top = (held.min[2]..=held.max[2])
        .rev()
        .find(|&z| !client.cells().cell(point.sector, [x, y, z]).is_air())
        .expect("a slab over the column");
    Platform::over(x, y, build.platform_bits, top + 1)
}

/// A world with a platform of 64 cells a side laid where [`world`] stands,
/// a whole plot of it, and the first cell over the corner of its slab: the
/// tests speak in cells counted from there.
fn opened() -> (Client, Build, SurfacePoint, [i32; 3]) {
    let (mut client, point) = world();
    let mut build = Build::default();
    build.set_platform(64);
    lay_now(&mut build, &mut client, point, Base::Deck).expect("dry land");
    let platform = platform_at(&build, &client, point);
    let origin = [platform.corner[0], platform.corner[1], platform.top];
    (client, build, point, origin)
}

fn at(origin: [i32; 3], cell: [i32; 3]) -> [i32; 3] {
    [0, 1, 2].map(|i| origin[i] + cell[i])
}

/// The middle of the top of a cell.
fn top(sphere: QuadSphere, sector: Sector, cell: [i32; 3]) -> DVec3 {
    let low = corner(sphere, sector, [cell[0], cell[1], cell[2] + 1]);
    let high = corner(sphere, sector, [cell[0] + 1, cell[1] + 1, cell[2] + 1]);
    (low + high) / 2.0
}

/// The middle of a side of a cell.
fn side(sphere: QuadSphere, sector: Sector, cell: [i32; 3], face: Face) -> DVec3 {
    let quad = Quad {
        cell,
        face,
        paint: 0,
        open: [3; 4],
    };
    let corners = quad.corners().map(|at| corner(sphere, sector, at));
    corners.into_iter().sum::<DVec3>() / 4.0
}

/// An eye at `position` looking at `target`, the pointer in the middle.
fn eye_at(position: DVec3, target: DVec3) -> Eye {
    let look = (target - position).normalize();
    let right = look.cross(position.normalize()).normalize();
    let back = -look;
    let up = back.cross(right);
    Eye {
        position,
        rotation: DQuat::from_mat3(&DMat3::from_cols(right, up, back)),
        fov_y: 1.0,
        aspect: 1.5,
        pointer: [0.5, 0.5],
    }
}

/// Puts the pointer over a point of the world in front of the eye.
fn point_at(eye: &mut Eye, p: DVec3) {
    let local = eye.rotation.inverse() * (p - eye.position);
    let tan = (eye.fov_y / 2.0).tan();
    let [x, y] = [local.x / -local.z / tan, local.y / -local.z / tan];
    eye.pointer = [(x / eye.aspect + 1.0) / 2.0, (1.0 - y) / 2.0];
}

/// One turn of the tool, the pointer's button down or not, the key that
/// turns a stroke held or not.
fn update(build: &mut Build, client: &mut Client, eye: &Eye, using: bool, turning: bool) {
    let keys: &[&'static str] = if turning { &[key::TURN] } else { &[] };
    let turn = Turn {
        eye: *eye,
        using,
        keys,
        interrupted: false,
    };
    build.turn(&turn, &mut client.host(NAME));
}

/// A stroke from the top of one cell to the top of another, from high over
/// the middle of the two, looking down.
fn stroke(build: &mut Build, client: &mut Client, sector: Sector, from: [i32; 3], to: [i32; 3]) {
    let sphere = client.sphere();
    let (a, b) = (top(sphere, sector, from), top(sphere, sector, to));
    let middle = (a + b) / 2.0;
    let mut eye = eye_at(middle + middle.normalize() * 20.0, middle);
    point_at(&mut eye, a);
    update(build, client, &eye, true, false);
    point_at(&mut eye, b);
    update(build, client, &eye, true, false);
    update(build, client, &eye, false, false);
}

/// Every cell over the slab that is not air, in the volume the tests open,
/// counted from the first.
fn laid(client: &Client, point: SurfacePoint, origin: [i32; 3]) -> Vec<[i32; 3]> {
    let mut over = client
        .cells()
        .bounds_over(point.sector.into(), point)
        .expect("an open volume");
    over.min[2] = origin[2];
    over.cells()
        .filter(|&cell| !client.cells().cell(point.sector, cell).is_air())
        .map(|cell| [0, 1, 2].map(|i| cell[i] - origin[i]))
        .collect()
}

/// Cells laid on the slab, in the paint of the platform.
fn stand(client: &mut Client, sector: Sector, a: [i32; 3], b: [i32; 3]) {
    let gesture = voxel::Gesture::Create {
        span: Span::between(a, b),
        paint: 0,
    };
    client.host(NAME).apply(sector, &[gesture]).unwrap();
}

fn take(build: &mut Build, client: &mut Client, tool: Option<Tool>) {
    build.take(tool, &mut client.host(NAME));
}

/// The highest ground under the platform a body at `point` is given, in
/// cells: what its slab stands over when the feet are no higher.
fn highest_under(build: &Build, client: &mut Client, point: SurfacePoint) -> i32 {
    let (x, y) = (point.u.floor() as i32, point.v.floor() as i32);
    let square = Platform::over(x, y, build.platform_bits, 0);
    let side = square.side as i32;
    let mut highest = f64::MIN;
    for (dx, dy) in (0..=side).flat_map(|dy| (0..=side).map(move |dx| (dx, dy))) {
        let (u, v) = (square.corner[0] + dx, square.corner[1] + dy);
        highest = highest.max(ground(client, point.sector, f64::from(u), f64::from(v)));
    }
    highest.ceil() as i32
}

#[test]
fn a_platform_is_refused_on_the_edge_of_a_sector_and_with_no_word_of_a_world() {
    let (mut client, point) = world();
    let side = f64::from(client.sphere().blocks().side());
    let mut build = Build::default();
    for (u, v) in [(10.0, point.v), (point.u, side - 10.0), (side - 1.0, 1.0)] {
        let edge = SurfacePoint::new(point.sector, u, v);
        let asked = lay_now(&mut build, &mut client, edge, Base::Deck);
        assert_eq!(asked.err(), Some(Refusal::Seam), "{u} {v}");
    }
    // One plot in, the edge is no reason: the sea may be.
    let inside = SurfacePoint::new(point.sector, 70.0, side - 70.0);
    let asked = lay_now(&mut build, &mut client, inside, Base::Deck);
    assert_ne!(asked.err(), Some(Refusal::Seam));

    // A client no world has spoken to builds nothing (DECISIONS 104).
    let mut alone = Client::new(Recipe::new(1)).unwrap();
    let asked = lay_now(&mut build, &mut alone, point, Base::Deck);
    assert_eq!(asked.err(), Some(Refusal::Level));
    take(&mut build, &mut alone, Some(Tool::Create));
    assert_eq!(build.tool, None);
    assert!(!alone.pointing());
}

#[test]
fn a_platform_is_a_slab_over_the_highest_ground_on_pillars_down_to_it() {
    let (mut client, point) = world();
    let mut build = Build::default();
    build.set_platform(16);
    build.set_paint(6);
    assert_eq!(build.platform(), 16);
    let top_m = lay_now(&mut build, &mut client, point, Base::Deck).unwrap();
    let platform = platform_at(&build, &client, point);
    assert_eq!(f64::from(platform.top) * BLOCK_M, top_m);
    // The slab is whole, in the paint in hand, and no ground under it
    // stands over its top: the highest of it is within a cell of it.
    let slab = platform.slab();
    assert_eq!(slab.size(), [16, 16, 1]);
    let mut highest = f64::MIN;
    let mut pillars = 0;
    for [x, y, z] in slab.cells() {
        let cell = |at: [i32; 3]| client.cells().cell(point.sector, at);
        assert_eq!(cell([x, y, z]).paint(), Some(6), "{x} {y}");
        assert!(cell([x, y, z + 1]).is_air());
        // Under the slab it is open, but for a pillar, which reaches the
        // lowest ground at its foot and goes no deeper.
        let built: Vec<i32> = (z - 80..z).filter(|&z| !cell([x, y, z]).is_air()).collect();
        let corners = [(0, 0), (1, 0), (0, 1), (1, 1)].map(|(dx, dy)| {
            ground(
                &mut client,
                point.sector,
                f64::from(x + dx),
                f64::from(y + dy),
            )
        });
        highest = corners.into_iter().fold(highest, f64::max);
        let foot = corners.into_iter().fold(f64::MAX, f64::min).floor() as i32;
        if !built.is_empty() {
            pillars += 1;
            assert_eq!(built, (foot..z).collect::<Vec<_>>(), "{x} {y}");
        }
    }
    assert!((0.0..1.0).contains(&(f64::from(platform.top) - highest)));
    assert!(pillars <= 4);
    // One undo takes all of it back, and leaves the volume open.
    assert_eq!(client.cells().history(), (true, false));
    build.undo(&mut client.host(NAME));
    assert!(
        slab.cells()
            .all(|at| client.cells().cell(point.sector, at).is_air())
    );
    assert!(client.cells().covers(point.sector.into(), point));
}

#[test]
fn a_solid_base_fills_every_column_down_to_the_ground_on_a_steep_plot() {
    // The steepest plot found near the test world: 32 m of fall across
    // its 32 m, where a volume's bottom is most likely to cut a column.
    let (mut client, point) = world();
    let side = f64::from(client.sphere().blocks().side());
    let point = SurfacePoint::new(point.sector, side * 0.312, side * 0.372);
    let mut build = Build::default();
    build.set_platform(64);
    lay_now(&mut build, &mut client, point, Base::Solid).unwrap();
    let platform = platform_at(&build, &client, point);
    let mut deepest = 0;
    for [x, y, z] in platform.slab().cells() {
        let foot = [(0, 0), (1, 0), (0, 1), (1, 1)]
            .map(|(dx, dy)| {
                ground(
                    &mut client,
                    point.sector,
                    f64::from(x + dx),
                    f64::from(y + dy),
                )
            })
            .into_iter()
            .fold(f64::MAX, f64::min)
            .floor() as i32;
        deepest = deepest.max(z - foot);
        let cell = |at: [i32; 3]| client.cells().cell(point.sector, at);
        assert!(cell([x, y, foot - 1]).is_air(), "{x} {y}");
        for z in foot..=z {
            assert!(!cell([x, y, z]).is_air(), "{x} {y} {z}");
        }
    }
    assert!(deepest > 50, "{deepest}");
}

#[test]
fn the_side_picked_is_one_on_offer() {
    let mut build = Build::default();
    for (asked, given) in [(0, 8), (8, 8), (20, 16), (32, 32), (50, 64), (900, 64)] {
        build.set_platform(asked);
        assert_eq!(build.platform(), given, "{asked}");
    }
}

#[test]
fn platforms_of_one_size_meet_edge_to_edge() {
    let (mut client, point) = world();
    let mut build = Build::default();
    build.set_platform(16);
    lay_now(&mut build, &mut client, point, Base::Deck).unwrap();
    let first = platform_at(&build, &client, point).slab();
    let beside = SurfacePoint::new(point.sector, point.u + 16.0, point.v);
    lay_now(&mut build, &mut client, beside, Base::Deck).unwrap();
    let second = platform_at(&build, &client, beside).slab();
    assert_eq!(second.min[0], first.max[0] + 1);
    assert_eq!(second.min[1], first.min[1]);
}

#[test]
fn a_body_on_the_platform_stands_on_its_slab() {
    let (mut client, point) = world();
    let mut build = Build::default();
    let top_m = lay_now(&mut build, &mut client, point, Base::Deck).unwrap();
    let footing = client
        .cells()
        .footing(point.sector.into(), point, top_m)
        .unwrap();
    assert_eq!(footing.floor_m, Some(top_m));
    // The ground is under it everywhere, so it is the slab that holds.
    let feet_m = ground(&mut client, point.sector, point.u, point.v) * BLOCK_M;
    assert!(feet_m <= top_m);
}

#[test]
fn the_higher_of_the_ground_and_the_feet_says_how_high_a_slab_stands() {
    let cells = |top_m: f64| (top_m / BLOCK_M).round() as i32;

    // Feet on the ground, on a slope or not: the highest ground under
    // the square decides, and the slab stands over all of it.
    let (mut client, point) = world();
    let mut build = Build::default();
    let over = highest_under(&build, &mut client, point);
    let top = lay_now(&mut build, &mut client, point, Base::Deck).unwrap();
    assert_eq!(cells(top), over);

    // In the air over it, the feet decide: the slab is laid where the
    // body is, to the nearest cell.
    let (mut client, point) = world();
    let feet_m = f64::from(over) * BLOCK_M + 10.2;
    let top = lay_from(&mut build, &mut client, point, feet_m, Base::Deck).unwrap();
    assert_eq!(cells(top), over + 20);

    // Under the highest ground, in a dip of it, the ground decides still.
    let (mut client, point) = world();
    let feet_m = f64::from(over) * BLOCK_M - 3.0;
    let top = lay_from(&mut build, &mut client, point, feet_m, Base::Deck).unwrap();
    assert_eq!(cells(top), over);
}

#[test]
fn a_body_on_a_slab_is_given_the_next_one_level_with_it() {
    let (mut client, point) = world();
    let mut build = Build::default();
    let top = lay_now(&mut build, &mut client, point, Base::Deck).unwrap();
    // The next square along, asked for from the top of the first, a
    // hair to either side of the cell's edge.
    let side = f64::from(build.platform());
    let beside = SurfacePoint::new(point.sector, point.u + side, point.v);
    let over = highest_under(&build, &mut client, beside);
    let level = (top / BLOCK_M).round() as i32;
    for hair in [-1e-6, 1e-6] {
        let next = lay_from(&mut build, &mut client, beside, top + hair, Base::Deck).unwrap();
        assert_eq!((next / BLOCK_M).round() as i32, level.max(over));
    }
}

#[test]
fn a_floating_platform_is_its_slab_alone() {
    let (mut client, point) = world();
    let mut build = Build::default();
    let over = highest_under(&build, &mut client, point);
    let feet_m = f64::from(over) * BLOCK_M + 12.0;
    let top = lay_from(&mut build, &mut client, point, feet_m, Base::Floating).unwrap();
    let platform = platform_at(&build, &client, point);
    assert_eq!(f64::from(platform.top) * BLOCK_M, top);
    let held = client
        .cells()
        .bounds_over(point.sector.into(), point)
        .unwrap();
    let slab = platform.slab();
    let solid = (held.min[2]..=held.max[2])
        .flat_map(|z| slab.cells().map(move |[x, y, _]| [x, y, z]))
        .filter(|&at| !client.cells().cell(point.sector, at).is_air())
        .count();
    assert_eq!(solid, (platform.side * platform.side) as usize);
}

#[test]
fn a_platform_over_the_top_of_its_volume_is_refused() {
    let (mut client, point) = world();
    let mut build = Build::default();
    // A volume holds a tower's height of cells over the ground of its plot,
    // and a slab is the cell under its top.
    client.host(NAME).open(point.sector.into(), point).unwrap();
    let top = client
        .cells()
        .bounds_over(point.sector.into(), point)
        .unwrap()
        .max[2]
        + 1;
    let high = f64::from(top + 1) * BLOCK_M;
    let asked = lay_from(&mut build, &mut client, point, high, Base::Floating);
    assert_eq!(asked, Err(Refusal::High));
    assert!(build.laying.is_none());
    let near = f64::from(top) * BLOCK_M;
    assert!(lay_from(&mut build, &mut client, point, near, Base::Floating).is_ok());
}

#[test]
fn a_platform_is_read_over_turns_and_lifts_the_body_onto_it() {
    let (mut client, point) = world();
    client.teleport(point);
    let mut build = Build::default();
    build.set_platform(64);
    let before = client.host(NAME).feet();
    build.lay(Base::Deck, &mut client.host(NAME)).unwrap();
    assert!(build.busy());
    let here = corner(
        client.sphere(),
        point.sector,
        [point.u as i32, point.v as i32, 0],
    );
    let eye = eye_at(here + here.normalize() * 20.0, here);
    let mut turns = 0;
    while build.busy() {
        update(&mut build, &mut client, &eye, false, false);
        turns += 1;
    }
    assert_eq!(turns, 65_usize.div_ceil(GROUND_ROWS_PER_TURN));
    let after = client.host(NAME).feet();
    let slab = platform_at(&build, &client, before.point);
    assert_eq!(after.height_m, f64::from(slab.top) * BLOCK_M);
    assert!(after.height_m >= before.height_m);
}

#[test]
fn a_stroke_stands_over_two_neighbours() {
    let (mut client, mut build, point, origin) = opened();
    let beside = SurfacePoint::new(point.sector, point.u + 64.0, point.v);
    client
        .host(NAME)
        .open(beside.sector.into(), beside)
        .unwrap();
    take(&mut build, &mut client, Some(Tool::Create));
    build.set_paint(4);
    // From the slab out over the plot beside it, where there is none.
    stroke(
        &mut build,
        &mut client,
        point.sector,
        at(origin, [60, 20, -1]),
        at(origin, [67, 20, -1]),
    );
    for x in 60..=67 {
        let cell = client.cells().cell(point.sector, at(origin, [x, 20, 0]));
        assert_eq!(cell.paint(), Some(4), "{x}");
    }
    build.undo(&mut client.host(NAME));
    for x in 60..=67 {
        let cell = client.cells().cell(point.sector, at(origin, [x, 20, 0]));
        assert!(cell.is_air(), "{x}");
    }
}

#[test]
fn a_stroke_stops_where_no_volume_stands() {
    let (mut client, mut build, point, origin) = opened();
    take(&mut build, &mut client, Some(Tool::Create));
    stroke(
        &mut build,
        &mut client,
        point.sector,
        at(origin, [60, 20, -1]),
        at(origin, [67, 20, -1]),
    );
    let row: Vec<[i32; 3]> = (60..64).map(|x| [x, 20, 0]).collect();
    assert_eq!(laid(&client, point, origin), row);
}

#[test]
fn a_drag_on_the_slab_lays_a_row_in_the_paint_in_hand() {
    let (mut client, mut build, point, origin) = opened();
    take(&mut build, &mut client, Some(Tool::Create));
    build.set_paint(4);
    // Aimed at the slab, the new cells go on it.
    stroke(
        &mut build,
        &mut client,
        point.sector,
        at(origin, [10, 20, -1]),
        at(origin, [14, 20, -1]),
    );
    let cell = |x: i32| client.cells().cell(point.sector, at(origin, [x, 20, 0]));
    for x in 10..=14 {
        assert_eq!(cell(x).paint(), Some(4), "{x}");
    }
    assert!(cell(15).is_air());
}

#[test]
fn delete_and_paint_act_on_what_is_there() {
    let (mut client, mut build, point, origin) = opened();
    let sector = point.sector;
    take(&mut build, &mut client, Some(Tool::Create));
    stroke(
        &mut build,
        &mut client,
        sector,
        at(origin, [5, 5, -1]),
        at(origin, [7, 5, -1]),
    );
    take(&mut build, &mut client, Some(Tool::Paint));
    build.set_paint(9);
    stroke(
        &mut build,
        &mut client,
        sector,
        at(origin, [5, 5, 0]),
        at(origin, [5, 5, 0]),
    );
    take(&mut build, &mut client, Some(Tool::Delete));
    stroke(
        &mut build,
        &mut client,
        sector,
        at(origin, [7, 5, 0]),
        at(origin, [7, 5, 0]),
    );
    let cell = |x: i32| client.cells().cell(sector, at(origin, [x, 5, 0]));
    assert_eq!(cell(5).paint(), Some(9));
    assert_eq!(cell(6).paint(), Some(0));
    assert!(cell(7).is_air());
}

#[test]
fn the_slab_is_cells_like_any_other() {
    let (mut client, mut build, point, origin) = opened();
    take(&mut build, &mut client, Some(Tool::Delete));
    let slab = at(origin, [30, 30, -1]);
    stroke(&mut build, &mut client, point.sector, slab, slab);
    assert!(client.cells().cell(point.sector, slab).is_air());
}

#[test]
fn the_ghost_shows_the_stroke_and_goes_with_the_tool() {
    let (mut client, mut build, point, origin) = opened();
    let sphere = client.sphere();
    take(&mut build, &mut client, Some(Tool::Create));
    assert!(client.pointing());
    let slab = top(sphere, point.sector, at(origin, [3, 3, -1]));
    let eye = eye_at(slab + slab.normalize() * 5.0, slab);
    update(&mut build, &mut client, &eye, false, false);
    assert!(client.cells().ghost().is_some());
    take(&mut build, &mut client, None);
    update(&mut build, &mut client, &eye, false, false);
    assert_eq!(client.cells().ghost(), None);
    assert!(!client.pointing());
}

#[test]
fn a_click_lays_one_cell_however_low_the_eye() {
    let (mut client, mut build, point, origin) = opened();
    let sphere = client.sphere();
    let sector = point.sector;
    take(&mut build, &mut client, Some(Tool::Create));
    let slab = top(sphere, sector, at(origin, [20, 20, -1]));
    let back = corner(sphere, sector, at(origin, [14, 20, 0]))
        - corner(sphere, sector, at(origin, [20, 20, 0]));
    // Low and far: a plane read half a cell up would land cells away.
    let mut eye = eye_at(slab + back + slab.normalize() * 0.8, slab);
    point_at(&mut eye, slab);
    update(&mut build, &mut client, &eye, true, false);
    update(&mut build, &mut client, &eye, false, false);
    assert_eq!(laid(&client, point, origin), vec![[20, 20, 0]]);
}

/// A low eye eight cells back from a cell of the slab, the pointer on it,
/// that presses, moves the pointer up the view and lets go.
fn up_the_view(
    build: &mut Build,
    client: &mut Client,
    point: SurfacePoint,
    origin: [i32; 3],
    upright: bool,
) -> Vec<[i32; 3]> {
    let sphere = client.sphere();
    let sector = point.sector;
    let slab = top(sphere, sector, at(origin, [20, 20, -1]));
    let back = corner(sphere, sector, at(origin, [12, 20, 0]))
        - corner(sphere, sector, at(origin, [20, 20, 0]));
    let mut eye = eye_at(slab + back + slab.normalize() * 2.0, slab);
    point_at(&mut eye, slab);
    update(build, client, &eye, true, upright);
    eye.pointer[1] -= 0.2;
    update(build, client, &eye, true, upright);
    update(build, client, &eye, false, upright);
    laid(client, point, origin)
}

#[test]
fn a_drag_up_the_view_lays_a_slab_away_from_the_eye() {
    let (mut client, mut build, point, origin) = opened();
    take(&mut build, &mut client, Some(Tool::Create));
    let laid = up_the_view(&mut build, &mut client, point, origin, false);
    assert!(laid.len() >= 3, "{laid:?}");
    assert!(laid.iter().all(|at| at[2] == 0 && at[1] == 20), "{laid:?}");
}

#[test]
fn upright_the_same_drag_stands_a_wall() {
    let (mut client, mut build, point, origin) = opened();
    take(&mut build, &mut client, Some(Tool::Create));
    let laid = up_the_view(&mut build, &mut client, point, origin, true);
    assert!(laid.len() >= 3, "{laid:?}");
    assert!(laid.iter().all(|at| at[0] == 20 && at[1] == 20), "{laid:?}");
    assert_eq!(laid.iter().map(|at| at[2]).min(), Some(0));
}

#[test]
fn from_the_foot_of_a_wall_a_stroke_goes_up_the_wall() {
    let (mut client, mut build, point, origin) = opened();
    let sphere = client.sphere();
    let sector = point.sector;
    stand(
        &mut client,
        sector,
        at(origin, [30, 10, 0]),
        at(origin, [30, 20, 5]),
    );
    take(&mut build, &mut client, Some(Tool::Create));
    build.set_paint(4);
    // From the slab at the foot of the wall, onto the wall itself.
    let foot = top(sphere, sector, at(origin, [29, 15, -1]));
    let wall = side(sphere, sector, at(origin, [30, 12, 4]), Face::new(0, false));
    let back = corner(sphere, sector, at(origin, [14, 14, 0]))
        - corner(sphere, sector, at(origin, [29, 14, 0]));
    let mut eye = eye_at(foot + back + foot.normalize() * 6.0, foot);
    point_at(&mut eye, foot);
    update(&mut build, &mut client, &eye, true, false);
    point_at(&mut eye, wall);
    update(&mut build, &mut client, &eye, true, false);
    update(&mut build, &mut client, &eye, false, false);
    // A sheet against the wall, from the slab up to where it pointed.
    let sheet: Vec<[i32; 3]> = laid(&client, point, origin)
        .into_iter()
        .filter(|cell| cell[0] != 30)
        .collect();
    assert_eq!(sheet.len(), 4 * 5, "{sheet:?}");
    assert!(sheet.iter().all(|cell| cell[0] == 29), "{sheet:?}");
    assert!(sheet.contains(&[29, 12, 4]) && sheet.contains(&[29, 15, 0]));
}

#[test]
fn a_hand_turns_a_stroke_through_the_three_layers() {
    let (mut client, mut build, point, origin) = opened();
    let sphere = client.sphere();
    take(&mut build, &mut client, Some(Tool::Create));
    let from = top(sphere, point.sector, at(origin, [20, 20, -1]));
    let to = top(sphere, point.sector, at(origin, [24, 23, -1]));
    let mut eye = eye_at(from + from.normalize() * 20.0, from);
    point_at(&mut eye, from);
    update(&mut build, &mut client, &eye, true, false);
    point_at(&mut eye, to);
    let mut across = Vec::new();
    for turning in [false, true, false, true, false, true] {
        update(&mut build, &mut client, &eye, true, turning);
        let stroke = build.stroke.expect("a stroke being drawn");
        // Whichever way it lies, it is one cell thick, through its start.
        let (axis, _) = stroke.across;
        assert_eq!(stroke.end[axis], stroke.start[axis]);
        across.push(axis);
    }
    // On the slab it lies flat; turned, it stands one way, then the
    // other, and then lies flat again.
    assert_eq!(across[0], 2);
    assert_eq!(across[1], across[2]);
    assert_eq!(across[3], across[4]);
    assert_eq!(across[5], 2);
    let mut stood = [across[1], across[3]];
    stood.sort_unstable();
    assert_eq!(stood, [0, 1]);
}

#[test]
fn turned_a_stroke_from_the_side_of_a_pillar_lies_flat() {
    let (mut client, mut build, point, origin) = opened();
    let sphere = client.sphere();
    let sector = point.sector;
    stand(
        &mut client,
        sector,
        at(origin, [30, 30, 0]),
        at(origin, [30, 30, 6]),
    );
    take(&mut build, &mut client, Some(Tool::Create));
    build.set_paint(4);
    let face = Face::new(0, false);
    let from = side(sphere, sector, at(origin, [30, 30, 4]), face);
    // From high over it, the eye looks at the flat layer most squarely.
    let out = corner(sphere, sector, at(origin, [22, 30, 0]))
        - corner(sphere, sector, at(origin, [30, 30, 0]));
    let mut eye = eye_at(from + out + from.normalize() * 16.0, from);
    point_at(&mut eye, from);
    update(&mut build, &mut client, &eye, true, false);
    // Left alone it hugs the side it started on: a sheet, standing.
    let up = side(sphere, sector, at(origin, [29, 33, 6]), face);
    point_at(&mut eye, up);
    update(&mut build, &mut client, &eye, true, false);
    assert_eq!(build.stroke.unwrap().across.0, 0);
    // Turned once, it lies flat at the height it started at.
    let away = top(sphere, sector, at(origin, [25, 33, 4]));
    point_at(&mut eye, away);
    update(&mut build, &mut client, &eye, true, true);
    update(&mut build, &mut client, &eye, false, true);
    let shelf: Vec<[i32; 3]> = laid(&client, point, origin)
        .into_iter()
        .filter(|cell| cell[0] != 30)
        .collect();
    assert_eq!(shelf.len(), 5 * 4, "{shelf:?}");
    assert!(shelf.iter().all(|cell| cell[2] == 4), "{shelf:?}");
    assert!(shelf.contains(&[29, 30, 4]) && shelf.contains(&[25, 33, 4]));
    // The next stroke lies on its side again.
    update(&mut build, &mut client, &eye, false, false);
    assert_eq!(build.turned, 0);
}

#[test]
fn undo_takes_back_a_stroke_and_redo_puts_it_back() {
    let (mut client, mut build, point, origin) = opened();
    let sector = point.sector;
    take(&mut build, &mut client, Some(Tool::Create));
    stroke(
        &mut build,
        &mut client,
        sector,
        at(origin, [5, 5, -1]),
        at(origin, [7, 5, -1]),
    );
    let middle = at(origin, [6, 5, 0]);
    // The platform under it is a change to take back too.
    assert_eq!(client.cells().history(), (true, false));
    build.undo(&mut client.host(NAME));
    assert!(client.cells().cell(sector, middle).is_air());
    assert_eq!(client.cells().history(), (true, true));
    build.redo(&mut client.host(NAME));
    assert!(!client.cells().cell(sector, middle).is_air());
    // A new stroke forgets what was taken back.
    build.undo(&mut client.host(NAME));
    stroke(
        &mut build,
        &mut client,
        sector,
        at(origin, [9, 9, -1]),
        at(origin, [9, 9, -1]),
    );
    assert_eq!(client.cells().history(), (true, false));
}

#[test]
fn escape_drops_a_stroke_half_drawn_and_so_does_a_shell_that_lets_go() {
    let (mut client, mut build, point, origin) = opened();
    let sphere = client.sphere();
    take(&mut build, &mut client, Some(Tool::Create));
    let slab = top(sphere, point.sector, at(origin, [3, 3, -1]));
    let over = at(origin, [3, 3, 0]);
    let eye = eye_at(slab + slab.normalize() * 5.0, slab);
    update(&mut build, &mut client, &eye, true, false);
    assert!(build.cancel());
    update(&mut build, &mut client, &eye, false, false);
    assert!(client.cells().cell(point.sector, over).is_air());
    assert!(!build.cancel());

    // A window that loses focus mid stroke lands nothing either.
    update(&mut build, &mut client, &eye, true, false);
    assert!(build.stroke.is_some());
    let let_go = Turn {
        eye,
        using: false,
        keys: &[],
        interrupted: true,
    };
    build.turn(&let_go, &mut client.host(NAME));
    assert!(client.cells().cell(point.sector, over).is_air());
}

#[test]
fn a_platform_is_laid_on_the_moon_as_on_the_planet() {
    let (mut client, point) = world();
    let mut build = Build::default();
    // A column of the moon, on the moon's own grid.
    let side = f64::from(client.moon().blocks().side());
    let seat = Seat::Moon(point.sector);
    let column = SurfacePoint::new(point.sector, side * 0.41, side * 0.37);
    let ground = client.host(NAME).ground(seat, column);
    let feet = Feet {
        seat,
        point: column,
        height_m: ground * BLOCK_M,
    };
    let top = {
        let host = &mut client.host(NAME);
        build.lay_at(feet, Base::Deck, host).expect("dry land");
        build.read_ground(host, usize::MAX).expect("a platform")
    };
    assert!(f64::from(top) >= ground);
    assert!(client.cells().covers(seat, column));
    // The planet holds nothing at the column the same numbers name there.
    assert!(!client.cells().covers(Seat::Sector(point.sector), column));
    let (x, y) = (column.u.floor() as i32, column.v.floor() as i32);
    assert!(!client.cells().cell(seat, [x, y, top - 1]).is_air());
    // A body stands on the slab, and a cell of it is a block of the moon:
    // as wide as a block is, where the moon is.
    let top_m = f64::from(top) * BLOCK_M;
    let footing = client.cells().footing(seat, column, top_m).unwrap();
    assert_eq!(footing.floor_m, Some(top_m));
    let (a, b) = {
        let host = client.host(NAME);
        (
            host.corner(seat, [x, y, top]),
            host.corner(seat, [x + 1, y, top]),
        )
    };
    let wide = a.distance(b);
    assert!((0.3..0.7).contains(&wide), "a cell {wide} m wide");
    let from_centre = a.length() - client.moon().radius_m();
    assert!((from_centre - top_m).abs() < 1e-6, "{from_centre} {top_m}");
    // What was laid is taken back as on the planet.
    build.undo(&mut client.host(NAME));
    assert!(client.cells().cell(seat, [x, y, top - 1]).is_air());
}

#[test]
fn a_volume_opens_with_nothing_in_it_and_a_stroke_starts_on_its_ground() {
    let (mut client, point) = world();
    let mut build = Build::default();
    let seat = Seat::Sector(point.sector);
    // Asked for where the body stands, a volume opens and holds no cell.
    let feet = client.host(NAME).feet();
    build.command(OPEN, serde_json::json!({}), &mut client.host(NAME));
    let held = client.cells().bounds_over(feet.seat, feet.point);
    let held = held.expect("a volume under the body");
    assert!(client.cells().cell(feet.seat, held.min).is_air());
    assert_eq!(client.cells().history(), (false, false));

    // The same where the tests build, with an eye over it looking down.
    client.host(NAME).open(seat, point).unwrap();
    let (x, y) = (point.u.floor() as i32, point.v.floor() as i32);
    let under = ground(
        &mut client,
        point.sector,
        f64::from(x) + 0.5,
        f64::from(y) + 0.5,
    );
    let under = under.floor() as i32;
    let sphere = client.sphere();
    let target = top(sphere, point.sector, [x, y, under - 1]);
    let aside = corner(sphere, point.sector, [x + 6, y, under]) - target;
    let eye = eye_at(target + target.normalize() * 10.0 + aside, target);
    // To take away and to repaint, the ground is nothing to aim at.
    take(&mut build, &mut client, Some(Tool::Delete));
    update(&mut build, &mut client, &eye, false, false);
    assert_eq!(build.aim, None);
    // To create, it is where the first cell stands: the one that holds the
    // ground there.
    take(&mut build, &mut client, Some(Tool::Create));
    update(&mut build, &mut client, &eye, false, false);
    let aim = build.aim.expect("the ground of the volume");
    assert_eq!(aim.met, Met::Ground);
    let first = aim.hit.before();
    assert_eq!(first, [x, y, under]);
    assert!(
        client.cells().ghost().is_some(),
        "the cell it would make shows"
    );
    update(&mut build, &mut client, &eye, true, false);
    update(&mut build, &mut client, &eye, false, false);
    assert!(!client.cells().cell(seat, first).is_air());
    // The next stroke starts on what was built, as any does.
    update(&mut build, &mut client, &eye, false, false);
    let aim = build.aim.expect("the cell");
    assert_eq!((aim.met, aim.hit.cell), (Met::Cell, first));
    // And the first is taken back like any other.
    build.undo(&mut client.host(NAME));
    assert!(client.cells().cell(seat, first).is_air());
}

#[test]
fn a_stroke_starts_on_a_side_of_the_volume_it_is_drawn_in() {
    let (mut client, point) = world();
    let mut build = Build::default();
    let seat = Seat::Sector(point.sector);
    client.host(NAME).open(seat, point).unwrap();
    let room = client.cells().bounds_over(seat, point).unwrap();
    // From the middle of the plot, over the highest of its ground, looking
    // level at the side where it ends along `u`.
    // A volume holds 1024 cells over the highest ground of its plot.
    let highest = room.max[2] + 1 - 1024;
    let (x, y, z) = (room.max[0], (room.min[1] + room.max[1]) / 2, highest + 6);
    let middle = |host: &Host<'_>, [a, b, c]: [i32; 3], d: [i32; 3]| {
        (host.corner(seat, [a, b, c]) + host.corner(seat, d)) / 2.0
    };
    let eye = {
        let host = client.host(NAME);
        let from = middle(
            &host,
            [room.min[0] + 32, y, z],
            [room.min[0] + 33, y + 1, z + 1],
        );
        eye_at(from, middle(&host, [x + 1, y, z], [x + 1, y + 1, z + 1]))
    };
    // To take away, a side is nothing to aim at.
    take(&mut build, &mut client, Some(Tool::Delete));
    update(&mut build, &mut client, &eye, false, false);
    assert_eq!(build.aim, None);
    // To create, it is where the first cell stands, against it.
    take(&mut build, &mut client, Some(Tool::Create));
    update(&mut build, &mut client, &eye, false, false);
    let aim = build.aim.expect("the side of the volume");
    assert_eq!(aim.met, Met::Side);
    assert_eq!(aim.hit.before(), [x, y, z]);
    update(&mut build, &mut client, &eye, true, false);
    update(&mut build, &mut client, &eye, false, false);
    assert!(!client.cells().cell(seat, [x, y, z]).is_air());
    // What stands nearer than the side is met first.
    update(&mut build, &mut client, &eye, false, false);
    let aim = build.aim.expect("the cell");
    assert_eq!((aim.met, aim.hit.cell), (Met::Cell, [x, y, z]));
}
