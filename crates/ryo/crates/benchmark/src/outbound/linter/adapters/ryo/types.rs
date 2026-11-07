//! Plain data the documents of the ryo linter adapter exchange with its adapter.
use ryo_benchmark_contracts::domain::context::types::PreparedRuleContext;
use ryo_benchmark_contracts::domain::context::types::rule_observation_declarations::value::RuleObservationDeclarations;
use ryo_benchmark_contracts::domain::measurements::types::Invocation;
use ryo_benchmark_contracts::domain::workloads::types::{PreparedCase, Tool};

/// The encoded worker request, ready to be written to its request file.
#[derive(Clone, Debug)]
pub(crate) struct RequestJson(pub String);

/// The bytes a child printed as its report.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ReportBytes<'a>(pub &'a [u8]);

/// A materialized rule configuration whose canonical text is wanted.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RuleContextInput<'a>(pub &'a PreparedRuleContext);

/// Authored observation declarations whose wire value is wanted.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ObservationsInput<'a>(pub &'a RuleObservationDeclarations);

/// What a reference process's normalized settings are bound to.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SettingsInput<'a> {
    /// The invocation after normalization.
    pub normalized: &'a Invocation,
    /// The prepared case the invocation runs.
    pub case: &'a PreparedCase,
    /// The profile the case belongs to.
    pub profile: &'a Tool,
}
