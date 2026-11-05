//! Prepared inputs for the outbound SDK engine port.
use ryo::linter::RelativePath;
use std::collections::BTreeSet;

/// Selected missing inputs classified from the sealed request by the application.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparedMissingInputs {
    /// Resolved diagnostic cohort; dependency definitions never imply selection.
    pub selected_rules: Vec<ryo::linter::RuleId>,
    /// Requested paths observed absent by the sealed context; they have no source or metadata.
    pub selected_missing: BTreeSet<RelativePath>,
}

/// Plain projected inputs; these alone do not permit SDK execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngineConfiguration {
    pub selected_missing: BTreeSet<RelativePath>,
    pub selected_rules: BTreeSet<ryo::linter::RuleId>,
    pub selected_pairs: usize,
    pub hosted: Vec<ryo::linter::HostRelation>,
}

/// The concrete loaded request and its private SDK effect binding.
pub mod loaded_engine_session;

/// A checked consuming SDK execution handle.
pub mod configured_engine_session;
