//! Prepared inputs for the outbound SDK engine port.
use ryo::linter::RelativePath;
use std::collections::BTreeSet;

/// An authored query that maps selected provider diagnostics to a benchmark rule identity.
#[derive(Clone, Debug, PartialEq, Eq)]
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

/// What one verification keeps across the cases of its profile: the rules loaded once and the SDK engine every case runs over.
#[derive(Debug, Default)]
pub(crate) struct Retained {
    /// The rules loaded from the profile's query files.
    pub(crate) rules:
        std::sync::OnceLock<std::collections::BTreeMap<ryo::linter::RuleId, ryo::linter::Rule>>,
    /// The SDK engine every case runs over.
    pub(crate) linter: std::sync::OnceLock<ryo::linter::Linter>,
}
