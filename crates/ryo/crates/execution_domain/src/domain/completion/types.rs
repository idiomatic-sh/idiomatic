//! Scope and physical completeness evidence.
use crate::domain::coverage::types::{LackingKind, RuleIndex};
use ryo_relations::domain::relations::types::FileId;
use ryo_values::domain::plan_parts::types::ColumnIndex;
pub mod completion_policy;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ScopeIdentity {
    File { rule: RuleIndex, file: FileId },
    Project { rule: RuleIndex },
}
/// How completely a relation's rows were read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CoverageState {
    /// Every row was read.
    Complete,
    /// Partly read in these files (none: the repository as a whole). `lacks` says, for a file it names, which rows are missing;
    /// a file it does not name lacks any row.
    Partial {
        /// The files it was partly read in; none stands for the repository as a whole.
        files: Vec<Option<FileId>>,
        /// Which rows each named file lacks.
        lacks: Vec<FileLacks>,
    },
    /// Only files that no language reads, or whose syntax a rule skipped, lack input. They are out of scope for answers not keyed
    /// by file: they never make such an answer partial, yet they are never `Complete` for the rows keyed to them.
    Inapplicable {
        /// The files that lack input.
        files: Vec<FileId>,
    },
}

/// Which rows of an input are missing.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MissingRows {
    /// Any row could be.
    Any,
    /// Only rows whose `column` holds one of these kinds, sorted, once each.
    Kinds {
        /// The column whose kinds are lacking.
        column: ColumnIndex,
        /// The lacking kinds, sorted, once each.
        kinds: Vec<LackingKind>,
    },
}

/// The rows missing from one file of a partial input.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct FileLacks {
    /// The file; none for the repository as a whole.
    pub file: Option<FileId>,
    /// Which of its rows are missing.
    pub missing: MissingRows,
}

/// Whether incomplete physical facts permit observation publication.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservationAdmission {
    /// Observation publication is permitted.
    Ready,
    /// The physical facts are incomplete.
    Incomplete,
}
/// Whether physical evaluation inputs permit a region to run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegionAdmission {
    /// The region may run.
    Allowed,
    /// The region may not run.
    Blocked,
}
