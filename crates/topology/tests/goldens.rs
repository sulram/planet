//! Numbers the Go server mirrors: `server/internal/world/near_test.go` holds
//! the same points and the same values, so the actor measures who is near
//! with the projection the client stands on. Change one, change both.

use topology::{Grid, Sector, SurfacePoint};

#[test]
fn directions_the_server_mirrors() {
    let grid = Grid::new(16).expect("the reference size");
    let cases: [(u8, f64, f64, [f64; 3]); 5] = [
        (0, 32768.0, 32768.0, [1.0, 0.0, 0.0]),
        (
            2,
            40000.25,
            9001.5,
            [-0.533564823189042, 0.833083947757445, 0.145875684896822],
        ),
        (
            5,
            0.0,
            65536.0,
            [0.577350269189626, -0.577350269189626, -0.577350269189626],
        ),
        (
            3,
            65535.5,
            12.0,
            [0.577451717419852, -0.577465558208231, -0.577133470812531],
        ),
        (
            4,
            1000.0,
            64000.0,
            [-0.572553412021728, 0.558002585774256, 0.600679369257445],
        ),
    ];
    for (sector, u, v, want) in cases {
        let point = SurfacePoint::new(Sector::new(sector).unwrap(), u, v);
        let got = grid.direction(point);
        for axis in 0..3 {
            assert!(
                (got[axis] - want[axis]).abs() < 1e-12,
                "direction({sector}, {u}, {v}) = {got:?}, want {want:?}"
            );
        }
    }
}
