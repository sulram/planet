//! A MagicaVoxel file, read into one grid of voxels with its palette, and the
//! sides of those voxels that show (docs/sketches/entities.md).
//!
//! A `.vox` is a tree of chunks: `MAIN` holds one `SIZE` and `XYZI` for each
//! model, an `RGBA` palette, and a scene graph of transforms (`nTRN`),
//! groups (`nGRP`) and shapes (`nSHP`) that places the models. A file saved
//! before the scene graph existed has models and no nodes. Either way the
//! whole scene is flattened here into one [`Grid`], as the thing a person
//! made: one shape, moved and turned whole once it is placed in a world.
//!
//! This crate knows no sphere and reads no file: it takes bytes and gives a
//! grid and its quads. What is done with them, where a model stands in a
//! world and how it is drawn, is the client's. Everything is integers but
//! the mesh, so the same bytes give the same grid on every machine.

mod faces;

use std::collections::BTreeMap;

pub use faces::{Quad, Vertex, mesh};

/// The most voxels a side of the flattened grid may have: MagicaVoxel's own
/// limit on one model, and the size past which a thing is a build, not a
/// model (docs/sketches/entities.md § Budgets).
pub const SIDE: u32 = 256;

/// The colours of a `.vox`: the palette, as the file keeps it. A voxel says
/// an index `1..=255`, and `palette[index]` is its colour, `rgba`. Entry `0`
/// is nobody's.
pub type Palette = [[u8; 4]; 256];

/// Why bytes are no model.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Error {
    /// Not a `.vox`: the first four bytes are not `VOX `.
    NotVox,
    /// The bytes end before a chunk does.
    Truncated,
    /// No model with a voxel in it.
    Empty,
    /// A model, or the scene flattened, is wider than [`SIDE`] along an axis.
    TooLarge,
    /// The scene graph names a node or a model the file does not hold.
    Dangling,
}

/// The voxels of a `.vox`, flattened to one grid: `x` and `y` across and `z`
/// up, as MagicaVoxel has them, trimmed to the voxels there are. A voxel is
/// an index into the palette, or `0` for air.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Grid {
    /// Voxels along each axis, each `1..=SIDE`.
    pub size: [u32; 3],
    /// `x` fastest, then `y`, then `z`: `size[0] * size[1] * size[2]` of them.
    pub voxels: Vec<u8>,
}

impl Grid {
    /// The voxel at a point, air outside the grid.
    pub fn at(&self, p: [i32; 3]) -> u8 {
        let inside = (0..3).all(|i| 0 <= p[i] && p[i] < self.size[i] as i32);
        if inside {
            self.voxels[self.index([p[0] as u32, p[1] as u32, p[2] as u32])]
        } else {
            0
        }
    }

    fn index(&self, [x, y, z]: [u32; 3]) -> usize {
        ((z * self.size[1] + y) * self.size[0] + x) as usize
    }

    /// How many voxels are solid.
    pub fn count(&self) -> usize {
        self.voxels.iter().filter(|&&v| v != 0).count()
    }
}

/// A `.vox` read: its grid and the colours it names.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Model {
    pub grid: Grid,
    pub palette: Palette,
}

/// Reads a `.vox`.
pub fn parse(bytes: &[u8]) -> Result<Model, Error> {
    let mut reader = Reader::new(bytes);
    if reader.take(4)? != b"VOX " {
        return Err(Error::NotVox);
    }
    reader.u32()?; // The version. 150 and 200 read alike here.
    let main = reader.chunk()?;
    if &main.id != b"MAIN" {
        return Err(Error::NotVox);
    }
    let mut file = File::default();
    let mut children = Reader::new(main.children);
    while !children.done() {
        file.read(children.chunk()?)?;
    }
    file.flatten()
}

/// One chunk of the file: its four letter id, its own bytes and its children's.
struct Chunk<'a> {
    id: [u8; 4],
    content: &'a [u8],
    children: &'a [u8],
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Reader<'a> {
        Reader { bytes, at: 0 }
    }

    fn done(&self) -> bool {
        self.at >= self.bytes.len()
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], Error> {
        let end = self.at.checked_add(n).ok_or(Error::Truncated)?;
        let slice = self.bytes.get(self.at..end).ok_or(Error::Truncated)?;
        self.at = end;
        Ok(slice)
    }

    fn u32(&mut self) -> Result<u32, Error> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn i32(&mut self) -> Result<i32, Error> {
        self.u32().map(|n| n as i32)
    }

    fn chunk(&mut self) -> Result<Chunk<'a>, Error> {
        let id = self.take(4)?;
        let content = self.u32()? as usize;
        let children = self.u32()? as usize;
        Ok(Chunk {
            id: [id[0], id[1], id[2], id[3]],
            content: self.take(content)?,
            children: self.take(children)?,
        })
    }

    /// A string: its length, then its bytes.
    fn string(&mut self) -> Result<&'a [u8], Error> {
        let n = self.u32()? as usize;
        self.take(n)
    }

    /// A dictionary: how many pairs, then key and value strings.
    fn dict(&mut self) -> Result<BTreeMap<&'a [u8], &'a [u8]>, Error> {
        let n = self.u32()?;
        let mut dict = BTreeMap::new();
        for _ in 0..n {
            let key = self.string()?;
            let value = self.string()?;
            dict.insert(key, value);
        }
        Ok(dict)
    }
}

/// One model as the file keeps it: its box and the voxels in it.
struct Raw {
    size: [u32; 3],
    /// `x, y, z, palette index` each.
    voxels: Vec<[u8; 4]>,
}

/// A node of the scene graph.
enum Node {
    /// Moves and turns what is under it: a translation of whole voxels and
    /// one of the 24 rotations of a cube, as a signed permutation.
    Transform {
        child: u32,
        translation: [i32; 3],
        rotation: Rotation,
    },
    Group(Vec<u32>),
    /// Models, placed with their middle at the transform over them.
    Shape(Vec<u32>),
}

/// A rotation of the grid: `rows[i]` is which axis of the input gives output
/// axis `i`, and with which sign.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Rotation {
    rows: [(usize, i32); 3],
}

impl Rotation {
    const IDENTITY: Rotation = Rotation {
        rows: [(0, 1), (1, 1), (2, 1)],
    };

    /// The byte MagicaVoxel packs a rotation in: two bits for the column of
    /// the first row, two for the second, then a sign bit for each row.
    fn of_byte(byte: u8) -> Rotation {
        let first = (byte & 3) as usize;
        let second = ((byte >> 2) & 3) as usize;
        // The third column is the one left over. A byte that names the same
        // column twice is no rotation; it is read as the identity's third.
        let third = [0, 1, 2]
            .into_iter()
            .find(|&c| c != first && c != second)
            .unwrap_or(2);
        let sign = |bit: u8| if byte & (1 << bit) != 0 { -1 } else { 1 };
        Rotation {
            rows: [
                (first.min(2), sign(4)),
                (second.min(2), sign(5)),
                (third, sign(6)),
            ],
        }
    }

    fn apply(self, v: [i32; 3]) -> [i32; 3] {
        self.rows.map(|(axis, sign)| sign * v[axis])
    }

    /// `self` after `inner`: what turns by `inner` and then by `self`.
    fn then(self, inner: Rotation) -> Rotation {
        Rotation {
            rows: self.rows.map(|(axis, sign)| {
                let (inner_axis, inner_sign) = inner.rows[axis];
                (inner_axis, sign * inner_sign)
            }),
        }
    }
}

/// Where a thing stands in the scene: a translation and a rotation, applied
/// to a point of the thing in half voxels, so the middle of a voxel is a
/// whole number and a cube turned about its middle lands on voxels again.
#[derive(Clone, Copy)]
struct Frame {
    translation: [i32; 3],
    rotation: Rotation,
}

impl Frame {
    const ROOT: Frame = Frame {
        translation: [0; 3],
        rotation: Rotation::IDENTITY,
    };

    /// This frame, then a node's own under it.
    fn under(self, translation: [i32; 3], rotation: Rotation) -> Frame {
        let moved = self.rotation.apply(translation);
        Frame {
            translation: [0, 1, 2].map(|i| self.translation[i] + moved[i]),
            rotation: self.rotation.then(rotation),
        }
    }
}

#[derive(Default)]
struct File {
    models: Vec<Raw>,
    /// The box the next `XYZI` fills.
    size: Option<[u32; 3]>,
    palette: Option<Palette>,
    nodes: BTreeMap<u32, Node>,
}

impl File {
    fn read(&mut self, chunk: Chunk<'_>) -> Result<(), Error> {
        let mut r = Reader::new(chunk.content);
        match &chunk.id {
            b"SIZE" => {
                let size = [r.u32()?, r.u32()?, r.u32()?];
                if size.iter().any(|&n| n == 0 || n > SIDE) {
                    return Err(Error::TooLarge);
                }
                self.size = Some(size);
            }
            b"XYZI" => {
                let size = self.size.take().unwrap_or([SIDE; 3]);
                let n = r.u32()? as usize;
                let mut voxels = Vec::with_capacity(n);
                for _ in 0..n {
                    let v = r.take(4)?;
                    let inside = (0..3).all(|i| u32::from(v[i]) < size[i]);
                    if inside && v[3] != 0 {
                        voxels.push([v[0], v[1], v[2], v[3]]);
                    }
                }
                self.models.push(Raw { size, voxels });
            }
            b"RGBA" => {
                let mut palette = [[0; 4]; 256];
                // The file keeps colour `i` at entry `i - 1`.
                for entry in palette.iter_mut().skip(1) {
                    let c = r.take(4)?;
                    *entry = [c[0], c[1], c[2], c[3]];
                }
                self.palette = Some(palette);
            }
            b"nTRN" => {
                let id = r.u32()?;
                r.dict()?;
                let child = r.u32()?;
                r.i32()?; // Reserved.
                r.i32()?; // The layer.
                let frames = r.u32()?;
                let mut translation = [0; 3];
                let mut rotation = Rotation::IDENTITY;
                // A model animated over frames stands here as its first.
                for frame in 0..frames {
                    let dict = r.dict()?;
                    if frame != 0 {
                        continue;
                    }
                    if let Some(t) = dict.get(b"_t".as_slice()) {
                        translation = numbers(t);
                    }
                    if let Some(rot) = dict.get(b"_r".as_slice()) {
                        rotation = Rotation::of_byte(numbers::<1>(rot)[0] as u8);
                    }
                }
                self.nodes.insert(
                    id,
                    Node::Transform {
                        child,
                        translation,
                        rotation,
                    },
                );
            }
            b"nGRP" => {
                let id = r.u32()?;
                r.dict()?;
                let n = r.u32()?;
                let children = (0..n).map(|_| r.u32()).collect::<Result<_, _>>()?;
                self.nodes.insert(id, Node::Group(children));
            }
            b"nSHP" => {
                let id = r.u32()?;
                r.dict()?;
                let n = r.u32()?;
                let mut models = Vec::with_capacity(n as usize);
                for _ in 0..n {
                    models.push(r.u32()?);
                    r.dict()?;
                }
                self.nodes.insert(id, Node::Shape(models));
            }
            // PACK, MATL, LAYR, rOBJ, rCAM, NOTE, IMAP: nothing of the shape.
            _ => {}
        }
        Ok(())
    }

    /// Every model placed where the scene graph puts it, as one grid.
    fn flatten(self) -> Result<Model, Error> {
        if self.models.is_empty() {
            return Err(Error::Empty);
        }
        let mut placed: Vec<([i32; 3], u8)> = Vec::new();
        if self.nodes.is_empty() {
            // No scene graph: every model stands at the origin.
            for model in &self.models {
                self.place(model, Frame::ROOT, &mut placed);
            }
        } else {
            // The root is the node with the lowest id, 0 as MagicaVoxel
            // writes it.
            let root = *self.nodes.keys().next().ok_or(Error::Dangling)?;
            self.walk(root, Frame::ROOT, &mut placed, 0)?;
        }
        let palette = self.palette.unwrap_or_else(default_palette);
        Grid::of(placed).map(|grid| Model { grid, palette })
    }

    fn walk(
        &self,
        id: u32,
        frame: Frame,
        placed: &mut Vec<([i32; 3], u8)>,
        depth: u32,
    ) -> Result<(), Error> {
        // A graph with a cycle in it is no tree: nothing legitimate is this deep.
        if depth > 64 {
            return Err(Error::Dangling);
        }
        match self.nodes.get(&id).ok_or(Error::Dangling)? {
            Node::Transform {
                child,
                translation,
                rotation,
            } => self.walk(
                *child,
                frame.under(*translation, *rotation),
                placed,
                depth + 1,
            ),
            Node::Group(children) => {
                for &child in children {
                    self.walk(child, frame, placed, depth + 1)?;
                }
                Ok(())
            }
            Node::Shape(models) => {
                for &model in models {
                    let raw = self.models.get(model as usize).ok_or(Error::Dangling)?;
                    self.place(raw, frame, placed);
                }
                Ok(())
            }
        }
    }

    /// A model's voxels where a frame puts them. MagicaVoxel places a model
    /// by its middle, the size halved and rounded down, so each voxel is
    /// taken as its middle in half voxels, turned, moved, and put back.
    fn place(&self, raw: &Raw, frame: Frame, placed: &mut Vec<([i32; 3], u8)>) {
        let pivot = raw.size.map(|n| (n / 2) as i32);
        for &[x, y, z, color] in &raw.voxels {
            let local = [x, y, z].map(i32::from);
            // In half voxels, the middle of a voxel is `2 (v - pivot) + 1`.
            let middle = [0, 1, 2].map(|i| 2 * (local[i] - pivot[i]) + 1);
            let turned = frame.rotation.apply(middle);
            let at = [0, 1, 2].map(|i| (turned[i] + 2 * frame.translation[i]).div_euclid(2));
            placed.push((at, color));
        }
    }
}

impl Grid {
    /// The voxels placed, trimmed to their bounds. A voxel placed twice is the
    /// later one.
    fn of(placed: Vec<([i32; 3], u8)>) -> Result<Grid, Error> {
        let (first, _) = *placed.first().ok_or(Error::Empty)?;
        let (mut min, mut max) = (first, first);
        for &(at, _) in &placed {
            for i in 0..3 {
                min[i] = min[i].min(at[i]);
                max[i] = max[i].max(at[i]);
            }
        }
        let size = [0, 1, 2].map(|i| (max[i] - min[i] + 1) as u32);
        if size.iter().any(|&n| n > SIDE) {
            return Err(Error::TooLarge);
        }
        let mut grid = Grid {
            size,
            voxels: vec![0; (size[0] * size[1] * size[2]) as usize],
        };
        for (at, color) in placed {
            let i = grid.index([0, 1, 2].map(|i| (at[i] - min[i]) as u32));
            grid.voxels[i] = color;
        }
        Ok(grid)
    }
}

/// Whole numbers written as text with spaces between: `"-3 0 12"`. What the
/// scene graph says a translation is. Missing or unreadable ones are zero.
fn numbers<const N: usize>(text: &[u8]) -> [i32; N] {
    let mut out = [0; N];
    let text = core::str::from_utf8(text).unwrap_or("");
    for (slot, word) in out.iter_mut().zip(text.split_ascii_whitespace()) {
        *slot = word.parse().unwrap_or(0);
    }
    out
}

/// The palette a `.vox` with no `RGBA` chunk means: MagicaVoxel's own. The
/// 215 colours of the web cube, each channel one of six steps, black left
/// out, then four ramps of ten: red, green, blue and grey.
pub fn default_palette() -> Palette {
    const STEPS: [u8; 6] = [0xff, 0xcc, 0x99, 0x66, 0x33, 0x00];
    const RAMP: [u8; 10] = [0xee, 0xdd, 0xbb, 0xaa, 0x88, 0x77, 0x55, 0x44, 0x22, 0x11];
    let mut palette = [[0; 4]; 256];
    let mut colours = palette.iter_mut().skip(1);
    for r in STEPS {
        for g in STEPS {
            for b in STEPS {
                if (r, g, b) == (0, 0, 0) {
                    continue;
                }
                *colours.next().expect("255 colours") = [r, g, b, 0xff];
            }
        }
    }
    for v in RAMP {
        *colours.next().expect("255 colours") = [v, 0, 0, 0xff];
    }
    for v in RAMP {
        *colours.next().expect("255 colours") = [0, v, 0, 0xff];
    }
    for v in RAMP {
        *colours.next().expect("255 colours") = [0, 0, v, 0xff];
    }
    for v in RAMP {
        *colours.next().expect("255 colours") = [v, v, v, 0xff];
    }
    palette
}
