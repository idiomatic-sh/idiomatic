//! One native side's observation of one registered case.
use super::finding::{Finding, SkippedPair};
use super::reference_discrepancy::{ArtifactPointer, DiscrepancyValidation, ExpectationKind};
use super::text::{
    CanonicalJson, CaseName, PathText, ReasonText, RuleName, SchemaName, Sha256Text, Side, YesNo,
};
use ryo_benchmark_contracts::domain::reports::types::{
    ComparisonContract, ExecutionMetadata, RuleCoverage,
};

/// Whether the case's sources are staged alone or in a listed project context.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ContextMode {
    /// Only the case's own files.
    #[default]
    Isolated,
    /// The files of a declared project directory.
    ListedDisk,
}

/// The listed context a side actually ran in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextManifest {
    /// How the context was staged.
    pub mode: ContextMode,
    /// The root the sources were logically under.
    pub logical_root: PathText,
    /// The directory the sources actually ran under.
    pub actual_root: PathText,
    /// Framed digest of the context bytes.
    pub manifest_hash: Sha256Text,
}

/// The raw artifacts of a process-failure reference.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessArtifacts {
    /// The raw standard output.
    pub stdout: ArtifactPointer,
    /// The raw standard error.
    pub stderr: ArtifactPointer,
    /// The raw invocation record.
    pub invocation: ArtifactPointer,
}

/// The exit status or signal number of a process the reference ran.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExitStatus(pub i64);

/// The reference process a side observed instead of diagnostics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessObservation {
    /// The kind of expectation observed.
    pub kind: ExpectationKind,
    /// The numeric exit status.
    pub exit: ExitStatus,
    /// A terminating signal: present when the process was signalled.
    pub signal: Option<ExitStatus>,
    /// The directory the process ran under.
    pub actual_request_root: PathText,
    /// Whether the observer used no deadline.
    pub observer_no_deadline: YesNo,
    /// The executable that ran: absent when capturing metadata failed.
    pub executable_sha256: Option<Sha256Text>,
    /// The normalized invocation settings: absent when capturing metadata failed.
    pub settings_sha256: Option<Sha256Text>,
    /// Why capturing metadata failed: present when it did.
    pub metadata_error: Option<ReasonText>,
    /// Digest of the expected output template: present when the case registers a process failure.
    pub template_sha256: Option<Sha256Text>,
    /// The raw artifacts: attached by the receipt store when it persists the raw bytes.
    pub artifacts: Option<ProcessArtifacts>,
}

/// A native side's durable observation row.
#[derive(Clone, Debug)]
pub struct ObservationDocument {
    /// The rule observed.
    pub rule: RuleName,
    /// The case observed.
    pub name: CaseName,
    /// The contestant.
    pub side: Side,
    /// Whether the observation completed.
    pub complete: YesNo,
    /// Why it did not: present when verification failed.
    pub error: Option<ReasonText>,
    /// Whether the side met its expectation.
    pub case_passed: YesNo,
    /// The sources the side read.
    pub files: Vec<PathText>,
    /// Digest of the sources: absent when the source identity was unavailable.
    pub source_sha256: Option<Sha256Text>,
    /// Digest of the declared file metadata: written when the case declares file metadata.
    pub file_metadata_sha256: Option<Sha256Text>,
    /// Digest of the declared project inputs: written when the case declares project inputs.
    pub project_inputs_sha256: Option<Sha256Text>,
    /// What the registered case expects.
    pub expected: Vec<Finding>,
    /// What the reference is expected to report: absent when it cannot be derived.
    pub reference_expected: Option<Vec<Finding>>,
    /// What the side reported: absent for a process failure or a failed run.
    pub diagnostics: Option<Vec<Finding>>,
    /// The rule/file pairs the side skipped: absent for a process failure or a failed run.
    pub skipped: Option<Vec<SkippedPair>>,
    /// The schema the row follows.
    pub observation_schema: SchemaName,
    /// The evidence the row was compared under.
    pub expectation_contract: ComparisonContract,
    /// What the producer can establish about execution: absent without a report.
    pub execution_metadata: Option<ExecutionMetadata>,
    /// Rule/file accounting: absent when the producer cannot account.
    pub coverage: Option<RuleCoverage>,
    /// The listed context the side ran in: present in a listed-disk run.
    pub context_manifest: Option<ContextManifest>,
    /// The project context receipt, compared for equality: present when the report carries one.
    pub project_context: Option<CanonicalJson>,
    /// The prepared rule context, compared for equality: written when the case declares one.
    pub rule_context: Option<CanonicalJson>,
    /// The declared rule observations, compared for equality: written when the case declares them.
    pub rule_observations: Option<CanonicalJson>,
    /// An explicit rule cohort: written when the case declares one.
    pub selected_rules: Option<Vec<RuleName>>,
    /// The directory the request ran under: written by the current writer.
    pub actual_request_root: Option<PathText>,
    /// Whether a documented language defect was recorded.
    pub documented_reference_discrepancy: YesNo,
    /// Whether a documented tool contract difference was recorded: absent in rows written before it existed.
    pub documented_tool_contract_difference: Option<YesNo>,
    /// What the side attests it validated: present for a documented discrepancy.
    pub discrepancy_validation: Option<DiscrepancyValidation>,
    /// Whether the raw report matched the intended behavior: absent without a report.
    pub raw_parity_matches_intended: Option<YesNo>,
    /// Whether the diagnostics matched the intended behavior: absent without a report.
    pub diagnostics_match_intended: Option<YesNo>,
    /// The reference process: present when it failed instead of reporting.
    pub process_observation: Option<ProcessObservation>,
}
