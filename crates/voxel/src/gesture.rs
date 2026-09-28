//! A gesture: one stroke of a build tool, over a box of cells.
//!
//! A drag is one gesture however many cells it covers, which is what an op
//! carries once edits are sent and logged (DECISIONS 59): the shape of the
//! stroke and its paint, never the thousands of cells it wrote.

use crate::{Cell, Span, Volume};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Gesture {
    /// Fills the air of a box with one paint. What is already solid stays as
    /// it is: creating never repaints.
    Create { span: Span, paint: u8 },
    /// Empties a box.
    Delete { span: Span },
    /// Repaints what is solid in a box. Air stays air: painting across a wall
    /// with a window in it leaves the window.
    Paint { span: Span, paint: u8 },
}

impl Gesture {
    pub fn span(self) -> Span {
        match self {
            Gesture::Create { span, .. }
            | Gesture::Delete { span }
            | Gesture::Paint { span, .. } => span,
        }
    }
}

impl Volume {
    /// Applies a gesture, clipped to the box. Returns the box around every
    /// cell it changed, `None` when it changed nothing.
    pub fn apply(&mut self, gesture: Gesture) -> Option<Span> {
        let span = gesture.span().meet(self.bounds())?;
        let mut changed: Option<Span> = None;
        for at in span.cells() {
            let old = self.get(at);
            let new = match gesture {
                Gesture::Create { paint, .. } if old.is_air() => Cell::solid(paint),
                Gesture::Delete { .. } => Cell::AIR,
                Gesture::Paint { paint, .. } if !old.is_air() => Cell::solid(paint),
                _ => continue,
            };
            if self.set(at, new) {
                changed = Some(changed.map_or(Span::cell(at), |span| span.with(at)));
            }
        }
        changed
    }
}

impl Volume {
    /// The cells of a box clipped to this one, in [`Span::cells`] order: what
    /// a gesture is about to change, kept to take it back.
    pub fn cells(&self, span: Span) -> Vec<Cell> {
        span.meet(self.bounds())
            .map(|span| span.cells().map(|at| self.get(at)).collect())
            .unwrap_or_default()
    }

    /// Puts back cells [`Volume::cells`] read from the same box. Returns the
    /// box around every cell it changed, `None` when it changed nothing.
    pub fn restore(&mut self, span: Span, cells: &[Cell]) -> Option<Span> {
        let span = span.meet(self.bounds())?;
        let mut changed: Option<Span> = None;
        for (at, &cell) in span.cells().zip(cells) {
            if self.set(at, cell) {
                changed = Some(changed.map_or(Span::cell(at), |span| span.with(at)));
            }
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(volume: &Volume) -> Vec<Option<u8>> {
        (0..4).map(|x| volume.get([x, 0, 0]).paint()).collect()
    }

    #[test]
    fn create_fills_air_and_leaves_what_stands() {
        let mut volume = Volume::new([4, 4, 4]);
        volume.apply(Gesture::Create {
            span: Span::cell([1, 0, 0]),
            paint: 5,
        });
        let changed = volume.apply(Gesture::Create {
            span: Span::between([0, 0, 0], [3, 0, 0]),
            paint: 2,
        });
        assert_eq!(row(&volume), vec![Some(2), Some(5), Some(2), Some(2)]);
        assert_eq!(changed, Some(Span::between([0, 0, 0], [3, 0, 0])));
    }

    #[test]
    fn paint_leaves_air_as_air() {
        let mut volume = Volume::new([4, 4, 4]);
        for x in [0, 3] {
            volume.apply(Gesture::Create {
                span: Span::cell([x, 0, 0]),
                paint: 1,
            });
        }
        volume.apply(Gesture::Paint {
            span: Span::between([0, 0, 0], [3, 0, 0]),
            paint: 9,
        });
        assert_eq!(row(&volume), vec![Some(9), None, None, Some(9)]);
    }

    #[test]
    fn delete_empties_and_says_what_it_touched() {
        let mut volume = Volume::new([4, 4, 4]);
        volume.apply(Gesture::Create {
            span: Span::between([0, 0, 0], [3, 0, 0]),
            paint: 1,
        });
        let changed = volume.apply(Gesture::Delete {
            span: Span::between([2, 0, 0], [9, 3, 3]),
        });
        assert_eq!(row(&volume), vec![Some(1), Some(1), None, None]);
        assert_eq!(changed, Some(Span::between([2, 0, 0], [3, 0, 0])));
        assert_eq!(
            volume.apply(Gesture::Delete {
                span: Span::cell([2, 0, 0])
            }),
            None
        );
    }

    #[test]
    fn cells_read_before_a_gesture_take_it_back() {
        let mut volume = Volume::new([4, 4, 4]);
        volume.apply(Gesture::Create {
            span: Span::cell([1, 0, 0]),
            paint: 5,
        });
        let span = Span::between([0, 0, 0], [3, 0, 0]);
        let before = volume.cells(span);
        volume.apply(Gesture::Paint { span, paint: 2 });
        volume.apply(Gesture::Create { span, paint: 7 });
        let after = volume.cells(span);
        assert_eq!(volume.restore(span, &before), Some(span));
        assert_eq!(row(&volume), vec![None, Some(5), None, None]);
        volume.restore(span, &after);
        assert_eq!(row(&volume), vec![Some(7), Some(2), Some(7), Some(7)]);
        assert_eq!(volume.restore(span, &after), None);
    }

    #[test]
    fn a_gesture_outside_the_box_changes_nothing() {
        let mut volume = Volume::new([4, 4, 4]);
        let outside = Gesture::Create {
            span: Span::between([5, 5, 5], [8, 8, 8]),
            paint: 1,
        };
        assert_eq!(volume.apply(outside), None);
    }
}
