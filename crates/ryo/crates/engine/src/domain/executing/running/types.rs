//! The types of the `running` part.

use crate::domain::code::types::{CodeEntry, WordSlot};
use crate::domain::steps::types::MadeTableId;

/// Whether the executor runs a word, a test or a fill.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WordBuilt {
    /// It runs.
    Runs,
    /// It is not built yet: a rule that can reach it is not loaded.
    NotYet,
}

/// A place a rule can reach, where the walk of its code goes on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WalkPlace {
    /// A step program of a unit, from an entry to its `End`.
    Program {
        /// The entry.
        entry: CodeEntry,
        /// The unit it is counted to.
        unit: WordSlot,
    },
    /// A pipeline of a table's fill, from an entry to its `End`.
    Pipeline {
        /// The entry.
        entry: CodeEntry,
        /// The unit of the table.
        unit: WordSlot,
    },
    /// A made table: its fill, and the table its own need may ask.
    Table(MadeTableId),
}
