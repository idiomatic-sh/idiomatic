//! The types of the `down` use case: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::manifest::types::ProductRoot;
use crate::domain::operations::types::{PerfRecord, Record};
use crate::domain::ratchet::types::{Compaction, Dropped, Lowered, Writing};

/// What `down` is asked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DownRequest {
    /// Where to look for the product.
    pub start: ProductRoot,
    /// Whether the lock is written.
    pub writing: Writing,
}

/// What `down` answers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DownReport {
    /// The product.
    pub root: ProductRoot,
    /// The records dropped, or that would be.
    pub dropped: Vec<Dropped>,
    /// The records lowered.
    pub lowered: Vec<Lowered>,
    /// The new in-source ignores recorded as suppressed, or that would be.
    pub recorded: Vec<Record>,
    /// The perf records dropped, or that would be: their tool ran and no longer reports them.
    pub perf: Vec<PerfRecord>,
    /// Whether the lock was compacted.
    pub compaction: Compaction,
    /// Whether it was written.
    pub writing: Writing,
}
