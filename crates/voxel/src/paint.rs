//! What a cell is made of: a paint, one byte, said in three parts. A colour
//! of whatever palette the volume is shown with, how it takes the light, and
//! the line drawn around each of its sides.

/// How a paint takes the light.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum Finish {
    /// Solid colour, lit by what shines on it.
    #[default]
    Matte,
    /// See-through: what stands behind it shows, tinted. Solid to a body and
    /// to a hand all the same.
    Glass,
    /// Shines with its own colour, by night as by day, and lights what is
    /// near.
    Light,
}

/// The line drawn around each side of a cell.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum Edge {
    #[default]
    None,
    Black,
    White,
}

/// A paint, taken apart.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct Paint {
    /// Which colour of the palette, under [`Paint::COLORS`].
    pub color: u8,
    pub finish: Finish,
    pub edge: Edge,
}

impl Paint {
    /// The colours a paint is one of.
    pub const COLORS: u8 = 16;
    const EDGES: u8 = 3;

    /// A matte paint of one colour, with no edge: the byte of it is the
    /// colour's own index.
    pub const fn plain(color: u8) -> Paint {
        Paint {
            color: color % Paint::COLORS,
            finish: Finish::Matte,
            edge: Edge::None,
        }
    }

    /// The paint a byte says.
    pub const fn of(byte: u8) -> Paint {
        let rest = byte / Paint::COLORS;
        Paint {
            color: byte % Paint::COLORS,
            finish: match rest / Paint::EDGES {
                1 => Finish::Glass,
                2 => Finish::Light,
                _ => Finish::Matte,
            },
            edge: match rest % Paint::EDGES {
                1 => Edge::Black,
                2 => Edge::White,
                _ => Edge::None,
            },
        }
    }

    /// The byte that says it: what a cell holds and the wire carries.
    pub const fn byte(self) -> u8 {
        let finish = match self.finish {
            Finish::Matte => 0,
            Finish::Glass => 1,
            Finish::Light => 2,
        };
        let edge = match self.edge {
            Edge::None => 0,
            Edge::Black => 1,
            Edge::White => 2,
        };
        self.color % Paint::COLORS + Paint::COLORS * (edge + Paint::EDGES * finish)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_paint_is_one_byte_and_a_plain_one_is_its_colour() {
        for color in 0..Paint::COLORS {
            assert_eq!(Paint::plain(color).byte(), color);
            for finish in [Finish::Matte, Finish::Glass, Finish::Light] {
                for edge in [Edge::None, Edge::Black, Edge::White] {
                    let paint = Paint {
                        color,
                        finish,
                        edge,
                    };
                    assert_eq!(Paint::of(paint.byte()), paint);
                }
            }
        }
        // Every byte a cell can hold is a paint, and none is past a cell.
        let most = Paint {
            color: Paint::COLORS - 1,
            finish: Finish::Light,
            edge: Edge::White,
        };
        assert!(usize::from(most.byte()) < crate::Cell::PAINTS);
        assert_eq!(Paint::of(250).finish, Finish::Matte);
    }
}
