//! Sectors and their integer frames.

type IVec3 = [i32; 3];

/// One of the six faces of the quad sphere.
///
/// Order: `+X, -X, +Y, -Y, +Z, -Z`. Each sector has a right handed integer
/// frame `(n, u, v)` with `u x v = n`, `n` pointing out of the planet.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct Sector(u8);

/// `[n, u, v]` per sector.
const FRAMES: [[IVec3; 3]; 6] = [
    [[1, 0, 0], [0, 1, 0], [0, 0, 1]],
    [[-1, 0, 0], [0, 0, 1], [0, 1, 0]],
    [[0, 1, 0], [0, 0, 1], [1, 0, 0]],
    [[0, -1, 0], [1, 0, 0], [0, 0, 1]],
    [[0, 0, 1], [1, 0, 0], [0, 1, 0]],
    [[0, 0, -1], [0, 1, 0], [1, 0, 0]],
];

impl Sector {
    pub const COUNT: usize = 6;
    pub const ALL: [Sector; 6] = [
        Sector(0),
        Sector(1),
        Sector(2),
        Sector(3),
        Sector(4),
        Sector(5),
    ];

    pub fn new(index: u8) -> Option<Sector> {
        (usize::from(index) < Self::COUNT).then_some(Sector(index))
    }

    pub fn index(self) -> usize {
        usize::from(self.0)
    }

    pub(crate) fn normal(self) -> IVec3 {
        FRAMES[self.index()][0]
    }

    pub(crate) fn u_axis(self) -> IVec3 {
        FRAMES[self.index()][1]
    }

    pub(crate) fn v_axis(self) -> IVec3 {
        FRAMES[self.index()][2]
    }

    /// The sector whose outward normal is the signed unit axis `n`.
    pub(crate) fn from_normal(n: IVec3) -> Sector {
        let found = Sector::ALL.into_iter().find(|s| s.normal() == n);
        found.expect("a signed unit axis")
    }

    /// The sector a direction points into: the axis with the largest magnitude.
    pub(crate) fn containing(d: [f64; 3]) -> Sector {
        let mut axis = 0;
        for i in 1..3 {
            if d[i].abs() > d[axis].abs() {
                axis = i;
            }
        }
        let mut n = [0; 3];
        n[axis] = if d[axis] < 0.0 { -1 } else { 1 };
        Sector::from_normal(n)
    }
}

/// A step along the grid of a sector.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Dir {
    UPos,
    UNeg,
    VPos,
    VNeg,
}

impl Dir {
    pub const ALL: [Dir; 4] = [Dir::UPos, Dir::UNeg, Dir::VPos, Dir::VNeg];

    pub fn opposite(self) -> Dir {
        match self {
            Dir::UPos => Dir::UNeg,
            Dir::UNeg => Dir::UPos,
            Dir::VPos => Dir::VNeg,
            Dir::VNeg => Dir::VPos,
        }
    }

    /// `(du, dv)` of one step.
    pub fn delta(self) -> (i32, i32) {
        match self {
            Dir::UPos => (1, 0),
            Dir::UNeg => (-1, 0),
            Dir::VPos => (0, 1),
            Dir::VNeg => (0, -1),
        }
    }

    pub(crate) fn from_delta(du: i32, dv: i32) -> Dir {
        match (du, dv) {
            (1, 0) => Dir::UPos,
            (-1, 0) => Dir::UNeg,
            (0, 1) => Dir::VPos,
            (0, -1) => Dir::VNeg,
            _ => unreachable!("not a unit grid step: ({du}, {dv})"),
        }
    }
}

pub(crate) fn idot(a: IVec3, b: IVec3) -> i32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
