//! What a corpus project's run request is built from.
use ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode;
use ryo_benchmark_contracts::domain::requests::types::{CohortSelection, SearchRoots};
use ryo_benchmark_contracts::domain::workloads::types::RuleContextProfile;

/// The rule side of every request over a corpus: the cohort, its configuration, its import roots and its context mode.
#[derive(Clone, Copy, Debug)]
pub struct ProjectCorpus<'a> {
    /// The rule cohort sent to both contestants.
    pub cohort: &'a CohortSelection,
    /// The profile's rule configuration declaration.
    pub rule_context: &'a RuleContextProfile,
    /// The ordered import roots of every project run.
    pub search_roots: &'a SearchRoots,
    /// How each project's context is sealed.
    pub context_mode: ProjectContextMode,
}
