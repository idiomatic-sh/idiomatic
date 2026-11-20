//! The `programs` service: what the host reads of a rule set's programs.

use super::types::Programs;
use crate::domain::scratch::types::{ScanScratch, ScratchEngine};
use ryo_language::domain::facts::service::file_words;
use ryo_language::inbound::use_cases::walk::types::RelationNeeds;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;

/// The relations some program of the rule set reads: what the host hands every walk, so a language builds only what is read.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::programs::service::needs;
/// use ryo_engine::domain::programs::types::Programs;
///
/// assert!(needs(&Programs::default()).0.is_empty());
/// ```
#[must_use]
pub fn needs(programs: &Programs) -> RelationNeeds<'_> {
    RelationNeeds(&programs.0.reads)
}

/// The rules the rule set runs, in rule order: the ones `load` accepted.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::programs::service::accepted;
/// use ryo_engine::domain::programs::types::Programs;
///
/// assert!(accepted(&Programs::default()).is_empty());
/// ```
#[must_use]
pub fn accepted(programs: &Programs) -> Vec<SelectedRuleOrdinal> {
    programs.0.rules.iter().map(|rule| rule.rule).collect()
}

/// A worker's per-file state for `programs`: its words over the programs' constants.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::programs::types::Programs;
/// use ryo_engine::domain::programs::service::scratch_for;
///
/// let _scratch = scratch_for(&Programs::default());
/// ```
#[must_use]
pub fn scratch_for(programs: &Programs) -> ScanScratch {
    ScanScratch(ScratchEngine {
        words: file_words(&programs.0.words),
        ..ScratchEngine::default()
    })
}
