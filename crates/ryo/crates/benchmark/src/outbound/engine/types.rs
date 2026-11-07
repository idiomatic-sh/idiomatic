//! Prepared inputs for the outbound SDK engine port.
use ryo::linter::RelativePath;
use std::collections::BTreeSet;

/// An authored query that maps selected provider diagnostics to a benchmark rule identity.
#[derive(Clone, Debug)]
pub struct FrontendDiagnosticProjection {
    /// The benchmark/reference rule identity emitted for selected rows.
    pub rule: ryo::linter::RuleId,
    /// Nonblank RyoQL source selecting which provider diagnostics become findings; the linter compiles it at runtime.
    pub query: ryo::query::RyoQlQuery,
}

/// One completed SDK run: the canonical report and the project lookups it observed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngineRun {
    /// The report, without a project context receipt; the use case seals it.
    pub report: ryo_benchmark_contracts::domain::reports::types::RunReport,
    /// Every project lookup the run observed, for the context port to seal.
    pub dependencies: Vec<ryo::linter::ProjectDependency>,
}

/// How many rule and file pairs the request selected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SelectedPairs(pub usize);

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
    pub selected_pairs: SelectedPairs,
    pub hosted: Vec<ryo::linter::HostRelation>,
}

/// The concrete loaded request and its private SDK effect binding.
pub mod loaded_engine_session;

/// A checked consuming SDK execution handle.
pub mod configured_engine_session;
