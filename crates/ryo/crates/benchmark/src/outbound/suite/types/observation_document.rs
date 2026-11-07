//! One native side's observation of one registered case.
use super::finding::{Finding, SkippedPair};
use super::reference_discrepancy::{ArtifactPointer, DiscrepancyValidation, ExpectationKind};
use super::text::{
    CanonicalJson, CaseName, PathText, ReasonText, RuleName, SchemaName, Sha256Text, ContestantSide, YesNo,
};
use ryo_benchmark_contracts::domain::reports::types::{
    ComparisonContract, ExecutionMetadata, RuleCoverage,
};

/// The identity of the case sources the side read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceIdentity {
    /// The sources were sealed under this digest.
    Sealed(Sha256Text),
    /// The source identity was unavailable.
    Unavailable,
}

/// The digest of the declared file metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileMetadataDigest {
    /// The case declares file metadata, under this digest.
    Declared(Sha256Text),
    /// The case declares no file metadata.
    Undeclared,
}

impl From<Option<Sha256Text>> for FileMetadataDigest {
    fn from(digest: Option<Sha256Text>) -> Self {
        digest.map_or(Self::Undeclared, Self::Declared)
    }
}

/// The digest of the declared project inputs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProjectInputsDigest {
    /// The case declares project inputs, under this digest.
    Declared(Sha256Text),
    /// The case declares no project inputs.
    Undeclared,
}

impl From<Option<Sha256Text>> for ProjectInputsDigest {
    fn from(digest: Option<Sha256Text>) -> Self {
        digest.map_or(Self::Undeclared, Self::Declared)
    }
}

/// What the reference is expected to report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReferenceExpectation {
    /// The reference expectation was derived.
    Derived(Vec<Finding>),
    /// The reference expectation cannot be derived.
    Underivable,
}

/// The listed context a side ran in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContextRun {
    /// The side ran on the case's own files.
    Isolated,
    /// The side ran in a listed-disk context.
    Listed(ContextManifest),
}

/// The project context receipt of the report, compared for equality.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProjectContextRecord {
    /// The report carries a project context.
    Carried(CanonicalJson),
    /// The report carries no project context.
    NotCarried,
}

/// The prepared rule context, compared for equality.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuleContextRecord {
    /// The case declares a rule context.
    Declared(CanonicalJson),
    /// The case declares no rule context.
    Undeclared,
}

/// The declared rule observations, compared for equality.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuleObservationsRecord {
    /// The case declares rule observations.
    Declared(CanonicalJson),
    /// The case declares no rule observations.
    Undeclared,
}

/// The explicit rule cohort of a case.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuleCohort {
    /// The case or a rule context declares this cohort.
    Declared(Vec<RuleName>),
    /// No cohort is declared.
    Undeclared,
}

/// Whether a documented tool contract difference was recorded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolContractRecord {
    /// The row records the answer.
    Recorded(YesNo),
    /// A row written before the answer existed.
    Unrecorded,
}

/// Whether the diagnostics matched the intended behavior.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntentMatch {
    /// The row records the answer.
    Matched(YesNo),
    /// The side produced no report to compare.
    Unreported,
}

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
    pub side: ContestantSide,
    /// Whether the observation completed.
    pub complete: YesNo,
    /// Why it did not: present when verification failed.
    pub error: Option<ReasonText>,
    /// Whether the side met its expectation.
    pub case_passed: YesNo,
    /// The sources the side read.
    pub files: Vec<PathText>,
    /// Digest of the sources.
    pub source_sha256: SourceIdentity,
    /// Digest of the declared file metadata.
    pub file_metadata_sha256: FileMetadataDigest,
    /// Digest of the declared project inputs.
    pub project_inputs_sha256: ProjectInputsDigest,
    /// What the registered case expects.
    pub expected: Vec<Finding>,
    /// What the reference is expected to report.
    pub reference_expected: ReferenceExpectation,
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
    /// The context the side ran in.
    pub context_manifest: ContextRun,
    /// The project context receipt.
    pub project_context: ProjectContextRecord,
    /// The prepared rule context.
    pub rule_context: RuleContextRecord,
    /// The declared rule observations.
    pub rule_observations: RuleObservationsRecord,
    /// The explicit rule cohort.
    pub selected_rules: RuleCohort,
    /// The directory the request ran under: written by the current writer.
    pub actual_request_root: Option<PathText>,
    /// Whether a documented language defect was recorded.
    pub documented_reference_discrepancy: YesNo,
    /// Whether a documented tool contract difference was recorded.
    pub documented_tool_contract_difference: ToolContractRecord,
    /// What the side attests it validated: present for a documented discrepancy.
    pub discrepancy_validation: Option<DiscrepancyValidation>,
    /// Whether the raw report matched the intended behavior: absent without a report.
    pub raw_parity_matches_intended: Option<YesNo>,
    /// Whether the diagnostics matched the intended behavior.
    pub diagnostics_match_intended: IntentMatch,
    /// The reference process: present when it failed instead of reporting.
    pub process_observation: Option<ProcessObservation>,
}
