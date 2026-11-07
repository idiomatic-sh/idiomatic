//! The types of the `up` use case: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::manifest::types::ProductRoot;
use crate::domain::operations::types::PerfRecord;
use crate::domain::operations::types::{Reason, Record};
use crate::domain::ratchet::types::{Compaction, Writing};
use crate::domain::selection::types::Selectors;
use crate::domain::violations::types::Violation;

/// What `up` is asked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpRequest {
    /// Where to look for the product.
    pub start: ProductRoot,
    /// Why the violations are accepted: it lands on every record.
    pub reason: Reason,
    /// Which new violations are recorded; none selects every one.
    pub selectors: Selectors,
    /// Whether the lock is written.
    pub writing: Writing,
}

/// What `up` answers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpReport {
    /// The product.
    pub root: ProductRoot,
    /// The records written, or that would be.
    pub recorded: Vec<Record>,
    /// The perf records written, or that would be, in the same operation.
    pub perf: Vec<PerfRecord>,
    /// The new violations the selectors left, which still fail `check`.
    pub left: Vec<Violation>,
    /// Whether the lock was compacted.
    pub compaction: Compaction,
    /// Whether it was written.
    pub writing: Writing,
}
