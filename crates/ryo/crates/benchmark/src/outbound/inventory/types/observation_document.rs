//! One native side's observation of one registered case.
use super::case_document::ContextMode;
use super::discrepancy::{ArtifactPointer, DiscrepancyValidation, ExpectationKind};
use super::finding::{Finding, SkippedPair};
use super::text::{
    CanonicalJson, CaseName, PathText, ReasonText, RuleName, SchemaName, Sha256Text, Side, YesNo,
};
use ryo_benchmark_contracts::domain::reports::types::{
    ComparisonContract, ExecutionMetadata, RuleCoverage,
};

/// The listed context a side actually ran in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextManifest {
    /// How the context was staged.
    pub mode: Option<ContextMode>,
    /// The root the sources were logically under.
    pub logical_root: Option<PathText>,
    /// The directory the sources actually ran under.
    pub actual_root: Option<PathText>,
    /// Framed digest of the context bytes.
    pub manifest_hash: Option<Sha256Text>,
}

/// What a process-failure reference observed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessArtifacts {
    /// The raw standard output.
    pub stdout: ArtifactPointer,
    /// The raw standard error.
    pub stderr: ArtifactPointer,
    /// The raw invocation record.
    pub invocation: ArtifactPointer,
}

/// The exit status of a process the reference ran.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExitStatus(pub i64);

/// The reference process a side observed instead of diagnostics.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProcessObservation {
    /// The kind of expectation observed.
    pub kind: Option<ExpectationKind>,
    /// The numeric exit status.
    pub exit: Option<ExitStatus>,
    /// A terminating signal, when the process was signalled.
    pub signal: Option<ExitStatus>,
    /// The directory the process ran under.
    pub actual_request_root: Option<PathText>,
    /// Whether the observer used no deadline.
    pub observer_no_deadline: Option<YesNo>,
    /// The executable that ran.
    pub executable_sha256: Option<Sha256Text>,
    /// The normalized invocation settings.
    pub settings_sha256: Option<Sha256Text>,
    /// Why capturing metadata failed, when it did.
    pub metadata_error: Option<ReasonText>,
    /// Digest of the expected output template.
    pub template_sha256: Option<Sha256Text>,
    /// The raw artifacts the receipt points at.
    pub artifacts: Option<ProcessArtifacts>,
}

/// A native side's durable observation row.
#[derive(Clone, Debug, Default)]
pub struct ObservationDocument {
    /// The rule observed.
    pub rule: Option<RuleName>,
    /// The case observed.
    pub name: Option<CaseName>,
    /// The contestant.
    pub side: Option<Side>,
    /// Whether the observation completed.
    pub complete: Option<YesNo>,
    /// Why it did not, when it did not.
    pub error: Option<ReasonText>,
    /// Whether the side met its expectation.
    pub case_passed: Option<YesNo>,
    /// The sources the side read.
    pub files: Option<Vec<PathText>>,
    /// Digest of the sources.
    pub source_sha256: Option<Sha256Text>,
    /// Digest of the declared file metadata.
    pub file_metadata_sha256: Option<Sha256Text>,
    /// Digest of the declared project inputs.
    pub project_inputs_sha256: Option<Sha256Text>,
    /// What the registered case expects.
    pub expected: Option<Vec<Finding>>,
    /// What the reference is expected to report.
    pub reference_expected: Option<Vec<Finding>>,
    /// What the side reported.
    pub diagnostics: Option<Vec<Finding>>,
    /// The rule/file pairs the side skipped.
    pub skipped: Option<Vec<SkippedPair>>,
    /// The schema the row follows.
    pub observation_schema: Option<SchemaName>,
    /// The evidence the row was compared under.
    pub expectation_contract: Option<ComparisonContract>,
    /// What the producer can establish about execution.
    pub execution_metadata: Option<ExecutionMetadata>,
    /// Rule/file accounting.
    pub coverage: Option<RuleCoverage>,
    /// The listed context the side ran in.
    pub context_manifest: Option<ContextManifest>,
    /// The project context receipt, compared for equality.
    pub project_context: Option<CanonicalJson>,
    /// The prepared rule context, compared for equality.
    pub rule_context: Option<CanonicalJson>,
    /// The declared rule observations, compared for equality.
    pub rule_observations: Option<CanonicalJson>,
    /// An explicit rule cohort.
    pub selected_rules: Option<Vec<RuleName>>,
    /// The directory the request ran under.
    pub actual_request_root: Option<PathText>,
    /// Whether a documented language defect was recorded.
    pub documented_reference_discrepancy: Option<YesNo>,
    /// Whether a documented tool contract difference was recorded.
    pub documented_tool_contract_difference: Option<YesNo>,
    /// What the side attests it validated for a discrepancy.
    pub discrepancy_validation: Option<DiscrepancyValidation>,
    /// Whether the raw report matched the intended behavior.
    pub raw_parity_matches_intended: Option<YesNo>,
    /// Whether the diagnostics matched the intended behavior.
    pub diagnostics_match_intended: Option<YesNo>,
    /// The reference process, when it failed instead of reporting.
    pub process_observation: Option<ProcessObservation>,
}
