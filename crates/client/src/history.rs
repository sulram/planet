//! The one history of what a hand changed (DECISIONS 115): every change
//! that landed, of every owner, in the order it landed, taken back and put
//! back as one list. The owners hold the changes themselves, here and in
//! the world; this holds which owner each was of, so a hand undoes the
//! last thing it did whatever it was, a stroke of cells or a part moved.
//!
//! In a world, each owner's take back is an op of its own, and the world
//! keeps each owner's stack in the same order this does, since a session's
//! ops land in the order asked. A change the world refused leaves this
//! list by the number it was asked under, so the two stay in step.

/// A system of the core that holds changes a hand takes back.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Owner {
    Cells,
    Parts,
}

/// One change that landed: whose, and under what number it was asked of
/// the world, while it has not answered.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Entry {
    owner: Owner,
    /// A set of one part, by its name here: sets of one part in a row are
    /// one change, as the world counts them.
    set_of: Option<u64>,
    ask: Option<u32>,
}

/// What a hand may take back, the last change last, and put back.
#[derive(Default)]
pub struct History {
    done: Vec<Entry>,
    undone: Vec<Entry>,
}

/// Changes kept to take back: as many as the owners keep.
const HISTORY: usize = 100;

impl History {
    /// A change landed, of an owner, asked of the world under `ask` when
    /// there is a world.
    pub(crate) fn did(&mut self, owner: Owner, ask: Option<u32>) {
        self.record(Entry {
            owner,
            set_of: None,
            ask,
        });
    }

    /// A part was set, by its name here: one change with the sets of the
    /// same part just before it.
    pub(crate) fn set(&mut self, part: u64, ask: Option<u32>) {
        if let Some(last) = self.done.last()
            && last.set_of == Some(part)
        {
            self.undone.clear();
            return;
        }
        self.record(Entry {
            owner: Owner::Parts,
            set_of: Some(part),
            ask,
        });
    }

    fn record(&mut self, entry: Entry) {
        self.undone.clear();
        self.done.push(entry);
        if self.done.len() > HISTORY {
            self.done.remove(0);
        }
    }

    /// The world refused the op asked under a number: a change it was is
    /// no change, and leaves the list.
    pub(crate) fn refused(&mut self, ask: u32) {
        self.done.retain(|entry| entry.ask != Some(ask));
        self.undone.retain(|entry| entry.ask != Some(ask));
    }

    /// Whose the last change to take back is, which moves to the ones to
    /// put back. `None` with none.
    pub(crate) fn take_back(&mut self) -> Option<Owner> {
        let entry = self.done.pop()?;
        self.undone.push(entry);
        Some(entry.owner)
    }

    /// Whose the last change to put back is, which moves to the ones to
    /// take back.
    pub(crate) fn put_back(&mut self) -> Option<Owner> {
        let entry = self.undone.pop()?;
        self.done.push(entry);
        Some(entry.owner)
    }

    /// Whether there is a change to take back, and one to put back.
    pub fn state(&self) -> (bool, bool) {
        (!self.done.is_empty(), !self.undone.is_empty())
    }

    /// A world is gone, or come: what it holds of the changes goes with it.
    pub(crate) fn clear(&mut self) {
        self.done.clear();
        self.undone.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changes_of_every_owner_go_back_in_order_and_sets_of_a_part_are_one() {
        let mut history = History::default();
        history.did(Owner::Cells, Some(1));
        history.did(Owner::Parts, Some(2));
        history.set(7, None);
        history.set(7, None);
        history.set(9, None);
        assert_eq!(history.state(), (true, false));
        assert_eq!(history.take_back(), Some(Owner::Parts));
        assert_eq!(
            history.take_back(),
            Some(Owner::Parts),
            "the two sets of 7 as one"
        );
        assert_eq!(history.take_back(), Some(Owner::Parts));
        assert_eq!(history.take_back(), Some(Owner::Cells));
        assert_eq!(history.take_back(), None);
        assert_eq!(history.put_back(), Some(Owner::Cells));
        assert_eq!(history.state(), (true, true));
        // A new change forgets what was to put back.
        history.did(Owner::Parts, Some(3));
        assert_eq!(history.state(), (true, false));
        // Refused, a change leaves the list.
        history.refused(3);
        assert_eq!(history.take_back(), Some(Owner::Cells));
    }
}
