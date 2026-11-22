//! The types of the `running` part.

use crate::domain::code::types::{CodeEntry, WordSlot};
use crate::domain::steps::types::MadeTableId;

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

/// The marks walks of the code leave on the words and the tables they meet, kept from one walk to the next (prototype
/// ryo-fmx3.1: each rule's walk made marks the size of the whole code): a word or a table is met in this walk when its
/// mark is the walk's number.
#[derive(Clone, Debug, Default)]
pub(crate) struct WalkMarks {
    /// The number of the walk under way; a mark of 0 was never met.
    pub(crate) walk: u32,
    /// The mark of each word (walked as a step, or either way when the kinds share their marks).
    pub(crate) words: Vec<u32>,
    /// The mark of each word walked as a stage, when the kinds keep their marks apart.
    pub(crate) stages: Vec<u32>,
    /// The mark of each table.
    pub(crate) tables: Vec<u32>,
}

/// Whether a walk marks a word walked as a step and as a stage alike (a rule's own walk) or apart (the walk of every
/// rule at once, which must meet each word in each way any rule's walk can).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum KindMarks {
    /// One mark a word.
    Shared,
    /// One mark a word and a way.
    Apart,
}
