//! Scope and physical completeness evidence.
use crate::domain::coverage::types::RuleIndex;
use ryo_relations::domain::relations::types::FileId;
pub(crate) mod completion_policy;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ScopeIdentity {
    File { rule: RuleIndex, file: FileId },
    Project { rule: RuleIndex },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CoverageState {
    Complete,
    Partial { files: Vec<Option<FileId>> },
    /// Only files that no language reads, or whose syntax a rule skipped, lack input. They are out of scope for answers not keyed
    /// by file: they never make such an answer partial, yet they are never `Complete` for the rows keyed to them.
    Inapplicable { files: Vec<FileId> },
}

/// Whether incomplete physical facts permit observation publication.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ObservationAdmission {
    Ready,
    Incomplete,
}
/// Whether evaluation must retain rows for physical coverage filtering.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RowRetention {
    Required,
    Unnecessary,
}
/// Whether physical evaluation inputs permit a region to run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RegionAdmission {
    Allowed,
    Blocked,
}

/// One actually admitted operand's schema and physical completeness. Missing
/// operands are intentionally absent when a lazy demand gate did not read them.
pub(crate) struct InputCoverage<'a> {
    pub ordinal: usize,
    pub kinds: &'a [ryo_relations::domain::relations::types::CellKind],
    pub coverage: CoverageState,
}
