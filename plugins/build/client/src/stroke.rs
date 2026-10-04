//! A stroke: one click and drag of a tool, from the cell it started on,
//! across a layer of cells through it, to the cell the pointer is over. It
//! lands as one gesture, and its ghost shows exactly the cells it would
//! change while it is drawn.

use client::{Aim, Cells, Eye, Host, Seat, Turn};
use glam::DVec3;
use voxel::{Gesture, Hit, Span};

use crate::{Build, Tool, key, refuse};

/// A stroke being drawn: from the cell it started on, across a layer of cells
/// through it, to the cell the pointer is over, or to where it meets that
/// layer when it is over nothing of it.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct Stroke {
    /// What the cells it is drawn in are seated on.
    pub seat: Seat,
    pub tool: Tool,
    pub start: [i32; 3],
    /// The side the stroke started on: the axis it faces, and where along it
    /// the surface is.
    pub side: (usize, f64),
    /// The axes of the three layers through the start, in the order a hand
    /// turns the stroke through them: the side it started on first.
    pub turns: [usize; 3],
    /// The axis the layer is across, and where along it the pointer is read.
    /// A stroke lies on the side it started on and is read on that side;
    /// turned, or led onto another surface, it lies across another axis and
    /// is read on its face toward the eye. Read anywhere else, the cell
    /// under the pointer is not the one the stroke shows.
    pub across: (usize, f64),
    pub end: [i32; 3],
}

impl Stroke {
    fn span(self) -> Span {
        Span::between(self.start, self.end)
    }

    /// The cell a tool takes where it aims: a new one goes in the air before
    /// the side hit, and what is taken away or repainted is the cell hit.
    fn cell(tool: Tool, hit: Hit) -> [i32; 3] {
        match tool {
            Tool::Create => hit.before(),
            Tool::Delete | Tool::Paint => hit.cell,
        }
    }

    fn gesture(self, paint: u8) -> Gesture {
        let span = self.span();
        match self.tool {
            Tool::Create => Gesture::Create { span, paint },
            Tool::Delete => Gesture::Delete { span },
            Tool::Paint => Gesture::Paint { span, paint },
        }
    }

    /// The stroke a tool starts where it aims, if it can start there, lying
    /// on the side it aims at.
    fn start(cells: &Cells, tool: Tool, aim: Aim) -> Option<Stroke> {
        let cell = Stroke::cell(tool, aim.hit);
        let face = aim.hit.face;
        let surface = f64::from(aim.hit.cell[face.axis] + i32::from(face.positive));
        cells.holds(aim.seat, cell).then_some(Stroke {
            seat: aim.seat,
            tool,
            start: cell,
            side: (face.axis, surface),
            turns: [0, 1, 2].map(|turn| (face.axis + turn) % 3),
            across: (face.axis, surface),
            end: cell,
        })
    }
}

impl Build {
    /// Aims, strokes and lands what the tool in hand does this turn. Each
    /// time the key that turns goes down, the stroke turns to the next of
    /// the three layers through its start.
    pub(crate) fn handle(&mut self, turn: &Turn<'_>, host: &mut Host<'_>) {
        let eye = &turn.eye;
        let turning = turn.held(key::TURN);
        let turned = turning && !self.turning;
        self.turning = turning;
        let Some(tool) = self.tool else {
            self.using = turn.using;
            return;
        };
        if turned {
            self.turned = (self.turned + 1) % 3;
        }
        let (from, toward) = eye.sight();
        let sight = host.sight(from, toward);
        self.aim = sight.aim();

        let pressed = turn.using && !self.using;
        let released = !turn.using && self.using;
        self.using = turn.using;
        if pressed {
            self.stroke = self.aim.and_then(|aim| {
                let mut stroke = Stroke::start(host.cells(), tool, aim)?;
                stroke.turns = turns(host, eye, stroke.seat, stroke.start, stroke.side.0);
                Some(stroke)
            });
        }
        if let Some(mut stroke) = self.stroke {
            let toward_eye = |axis: usize| near(host, eye, stroke.seat, stroke.start, axis);
            if pressed || turned {
                let axis = stroke.turns[self.turned];
                stroke.across = match axis == stroke.side.0 {
                    true => stroke.side,
                    false => (axis, toward_eye(axis)),
                };
            }
            // The cell the pointer is over, read as the start was. It ends
            // the stroke when it lies in its layer. A stroke no hand turned
            // also follows the pointer onto another surface its start lies
            // on, as from the foot of a wall up the wall.
            let (axis, at) = stroke.across;
            let over = self.aim.filter(|aim| aim.seat == stroke.seat);
            let led = over.and_then(|aim| {
                let cell = Stroke::cell(tool, aim.hit);
                let on = aim.hit.face.axis;
                if cell[axis] == stroke.start[axis] {
                    Some((stroke.across, cell))
                } else if self.turned == 0 && cell[on] == stroke.start[on] {
                    Some(((on, toward_eye(on)), cell))
                } else {
                    None
                }
            });
            match led {
                Some((across, cell)) => {
                    stroke.across = across;
                    stroke.end = cell;
                }
                // Over nothing of the layer, the pointer is read where its
                // line of sight crosses it, whatever is behind, and over as
                // many volumes as it reaches.
                None => {
                    if let Some(p) = sight.crossing(stroke.seat, axis, at) {
                        stroke.end = p.map(|n| n.floor() as i32);
                    }
                }
            }
            // Whatever it was before it turned, the stroke is one layer
            // thick: an end kept from another layer is brought into this one.
            let (axis, _) = stroke.across;
            stroke.end[axis] = stroke.start[axis];
            self.stroke = Some(stroke);
        }
        if released {
            self.turned = 0;
            if let Some(stroke) = self.stroke.take()
                && let Err(refusal) = host.apply(stroke.seat, &[stroke.gesture(self.paint)])
            {
                refuse(host, refusal.into());
            }
        }

        let preview = match (self.stroke, self.aim) {
            (Some(stroke), _) => Some(stroke),
            (None, Some(aim)) => Stroke::start(host.cells(), tool, aim),
            (None, None) => None,
        };
        host.preview(preview.map(|stroke| (stroke.seat, stroke.gesture(self.paint))));
    }
}

/// The step from a cell to the next along an axis, in the world.
fn step(host: &Host<'_>, seat: Seat, cell: [i32; 3], axis: usize) -> DVec3 {
    let mut next = cell;
    next[axis] += 1;
    host.corner(seat, next) - host.corner(seat, cell)
}

/// Where along an axis the pointer is read for the layer across it through
/// a cell: its face toward the eye, the low one when the eye looks up the
/// axis.
fn near(host: &Host<'_>, eye: &Eye, seat: Seat, cell: [i32; 3], axis: usize) -> f64 {
    let (_, forward) = eye.sight();
    f64::from(cell[axis] + i32::from(step(host, seat, cell, axis).dot(forward) < 0.0))
}

/// The axes of the three layers through a cell, in the order a hand turns a
/// stroke through them: the side it started on, then of the other two the
/// one the eye looks at more squarely, then the last.
fn turns(host: &Host<'_>, eye: &Eye, seat: Seat, cell: [i32; 3], side: usize) -> [usize; 3] {
    let (_, forward) = eye.sight();
    let depth = |axis: usize| {
        let along = step(host, seat, cell, axis).normalize_or_zero();
        along.dot(forward).abs()
    };
    let [p, q] = [(side + 1) % 3, (side + 2) % 3];
    match depth(p) >= depth(q) {
        true => [side, p, q],
        false => [side, q, p],
    }
}
