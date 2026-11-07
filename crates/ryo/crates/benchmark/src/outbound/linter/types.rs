//! Plain data of the linter port.
use ryo_benchmark_contracts::domain::context::types::PreparedRuleContext;
use ryo_benchmark_contracts::domain::context::types::rule_observation_declarations::value::RuleObservationDeclarations;
use ryo_benchmark_contracts::domain::measurements::types::Invocation;
use ryo_benchmark_contracts::domain::workloads::types::{PreparedCase, Tool};

/// The canonical text of one identity input of a reference process.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalText(pub String);

/// A materialized rule configuration whose canonical text is wanted.
#[derive(Clone, Copy, Debug)]
pub struct RuleContextInput<'a>(pub &'a PreparedRuleContext);

/// Authored observation declarations whose wire value is wanted.
#[derive(Clone, Copy, Debug)]
pub struct ObservationsInput<'a>(pub &'a RuleObservationDeclarations);

/// What a reference process's normalized settings are bound to.
#[derive(Clone, Copy, Debug)]
pub struct SettingsInput<'a> {
    /// The invocation after normalization.
    pub normalized: &'a Invocation,
    /// The prepared case the invocation runs.
    pub case: &'a PreparedCase,
    /// The profile the case belongs to.
    pub profile: &'a Tool,
}
