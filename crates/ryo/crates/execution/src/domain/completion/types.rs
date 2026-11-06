//! Scope and physical completeness evidence.
use crate::domain::coverage::types::{LackingKind, RuleIndex};
use ryo_relations::domain::relations::types::FileId;
use ryo_values::domain::plan_parts::types::ColumnIndex;
pub(crate) mod completion_policy;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ScopeIdentity {
    File { rule: RuleIndex, file: FileId },
    Project { rule: RuleIndex },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CoverageState {
    Complete,
    /// Partly read in these files (none: the repository as a whole). `lacks` says, for a file it names, which rows are missing;
    /// a file it does not name lacks any row.
    Partial {
        files: Vec<Option<FileId>>,
        lacks: Vec<FileLacks>,
    },
    /// Only files that no language reads, or whose syntax a rule skipped, lack input. They are out of scope for answers not keyed
    /// by file: they never make such an answer partial, yet they are never `Complete` for the rows keyed to them.
    Inapplicable {
        files: Vec<FileId>,
    },
}

/// Which rows of an input are missing.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum MissingRows {
    /// Any row could be.
    Any,
    /// Only rows whose `column` holds one of these kinds, sorted, once each.
    Kinds {
        column: ColumnIndex,
        kinds: Vec<LackingKind>,
    },
}

/// The rows missing from one file of a partial input.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct FileLacks {
    /// The file; none for the repository as a whole.
    pub file: Option<FileId>,
    /// Which of its rows are missing.
    pub missing: MissingRows,
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

/// One actually admitted operand's schema and physical completeness. MissingRows
/// operands are intentionally absent when a lazy demand gate did not read them.
pub(crate) struct InputCoverage<'a> {
    pub ordinal: usize,
    pub kinds: &'a [ryo_relations::domain::relations::types::CellKind],
    pub coverage: CoverageState,
}
