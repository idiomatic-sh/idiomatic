//! The types of the `ratchet` concept: plain data, public fields and derives, with no constructor and no inherent
//! impl.

use crate::domain::judgement::types::Resolution;
use crate::domain::operations::types::{Count, Record};
use crate::domain::renames::types::Changes;
use crate::domain::violations::types::RepoPath;

/// What `up` records.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Accepting {
    /// The records, one per key, each counting what was accepted under it before.
    pub records: Vec<Record>,
    /// How many new violations had no key and could not be accepted.
    pub unkeyed: Count,
}

/// Why `down` drops a record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DropReason {
    /// None of its violations occurs.
    Resolved(Resolution),
    /// Its file was renamed and the run reports no violation of its identity at the new path.
    RenamedWithoutFinding,
}

/// Where a dropped record's file went.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Moved {
    /// It was renamed to this path.
    To(RepoPath),
    /// Nowhere.
    Stayed,
}

/// A record `down` drops.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dropped {
    /// The record.
    pub record: Record,
    /// Where its file went.
    pub moved: Moved,
    /// Why.
    pub reason: DropReason,
}

/// A record `down` lowers: some of its violations still occur.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lowered {
    /// The record at its new count.
    pub record: Record,
    /// Its count before.
    pub from: Count,
}

/// What `down` does.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Lowering {
    /// The records dropped.
    pub dropped: Vec<Dropped>,
    /// The records lowered.
    pub lowered: Vec<Lowered>,
    /// What its operation adds and removes.
    pub changes: Changes,
}

/// Whether `up` and `down` write the lock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Writing {
    /// They do.
    Write,
    /// They say what they would write, and write nothing: `--dry-run`.
    DryRun,
}

/// Whether an operation compacted the lock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Compaction {
    /// A snapshot of it was appended, below every head.
    Snapshot,
    /// It was left as it is.
    Unchanged,
}
