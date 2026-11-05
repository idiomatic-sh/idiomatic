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
}
