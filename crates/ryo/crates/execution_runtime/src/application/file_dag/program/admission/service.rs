//! The `admission` application operation: the proof that a rule does not apply to a file, and the files a rule reads.

mod partitions;

use ryo_execution_domain::domain::{
    coverage::types::RuleIndex, rule_intent::types::ApplicabilityReason,
};
use ryo_relations::domain::relations::types::FileId;

/// The file a proof names.
#[derive(Clone)]
pub(crate) struct FileGeneration {
    pub(super) file: FileId,
}

/// Positive applicability is evidence, not a Complete-empty population.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_dag::program::admission::service::ApplicabilityProof;
/// fn named(_: Option<&ApplicabilityProof>) {}
/// ```
#[derive(Clone)]
pub struct ApplicabilityProof {
    pub(super) producer: RuleIndex,
    pub(super) file: FileGeneration,
    pub(super) reason: ApplicabilityReason,
}
