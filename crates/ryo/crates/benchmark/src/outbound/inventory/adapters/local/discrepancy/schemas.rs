//! The documented difference from the reference tool a case registers, with its findings and its
//! explicit process failure.
use crate::outbound::inventory::adapters::local::types::DiscrepancyText;
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::documents::types::finding::value::ExpectedFindings;
use ryo_benchmark_contracts::domain::documents::types::finding::value::{
    FindingOffset, RecordedFinding,
};
use ryo_benchmark_contracts::domain::documents::types::reference_discrepancy::value::{
    DiscrepancyIdentity, DiscrepancyRecord, ObservedExecutable, ProcessExpectation,
};
use ryo_benchmark_contracts::domain::documents::types::text::value::FileMetadataDigest;
use ryo_benchmark_contracts::domain::documents::types::text::value::{
    PathText, ReasonText, RuleName, Sha256Text,
};
use ryo_benchmark_contracts::domain::measurements::types::ProcessExitCode;
use ryo_benchmark_contracts::domain::process::types::{
    OutputPart, OutputText, PathEncoding, ProcessArtifact, ProcessFailure, ProcessObservation,
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::workloads::types::DiscrepancyBasis;
use serde::Deserialize;
use std::path::PathBuf;

/// The digest of an optional wire text.
fn sha(value: Option<String>) -> Option<Sha256Text> {
    value.map(Sha256Text)
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FindingSchema {
    rule: String,
    file: String,
    start: u64,
    end: u64,
}

impl From<FindingSchema> for RecordedFinding {
    fn from(wire: FindingSchema) -> Self {
        Self {
            rule: RuleName(wire.rule),
            file: PathText(wire.file),
            span: ryo_benchmark_contracts::domain::documents::types::finding::value::RecordedSpan {
                start: FindingOffset(wire.start),
                end: FindingOffset(wire.end),
            },
        }
    }
}

/// Every wire finding as a document finding.
fn findings(rows: Vec<FindingSchema>) -> Vec<RecordedFinding> {
    rows.into_iter().map(RecordedFinding::from).collect()
}

/// Only this explicit variant can request process-failure comparison.
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
enum ProcessFailureSchema {
    /// A completed numeric failure with exactly pinned byte streams.
    ProcessFailure {
        /// Exact nonzero numeric status, never a signal.
        exit: i32,
        /// Whole output template.
        stdout: Vec<OutputPartSchema>,
        /// Whole error template.
        stderr: Vec<OutputPartSchema>,
        /// Two captures for each exact supported executable.
        observations: Vec<ProcessObservationSchema>,
    },
}
/// Exact byte spelling or a declared source path in one explicit encoding.
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
enum OutputPartSchema {
    /// Unmodified UTF8 bytes, including whitespace.
    Literal { value: String },
    /// Authoritative invocation root joined to a declared input.
    InputPath {
        file: PathBuf,
        encoding: PathEncodingSchema,
    },
}
/// Output path encoding, without inferred normalization.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum PathEncodingSchema {
    /// Exact UTF8 bytes.
    Utf8Bytes,
    /// Canonical JSON string content, without surrounding quotes.
    JsonStringContent,
}
/// A raw file stored separately from independent semantic evidence.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct ProcessArtifactSchema {
    /// Explicit profile-relative path.
    path: PathBuf,
    /// Exact raw byte SHA256.
    sha256: String,
}
/// Independent captured observation grouped by exact executable digest.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct ProcessObservationSchema {
    /// No cross-architecture substitution is accepted.
    executable_sha256: String,
    /// Captured root/normalized settings/exit/source identity.
    invocation: ProcessArtifactSchema,
    /// Actual raw standard output.
    stdout: ProcessArtifactSchema,
    /// Actual raw standard error.
    stderr: ProcessArtifactSchema,
}

impl From<ProcessArtifactSchema> for ProcessArtifact {
    fn from(wire: ProcessArtifactSchema) -> Self {
        Self {
            path: wire.path,
            sha256: ArtifactDigest(wire.sha256),
        }
    }
}
impl From<OutputPartSchema> for OutputPart {
    fn from(wire: OutputPartSchema) -> Self {
        match wire {
            OutputPartSchema::Literal { value } => Self::Literal(OutputText(value)),
            OutputPartSchema::InputPath { file, encoding } => Self::InputPath {
                file,
                encoding: match encoding {
                    PathEncodingSchema::Utf8Bytes => PathEncoding::Utf8Bytes,
                    PathEncodingSchema::JsonStringContent => PathEncoding::JsonStringContent,
                },
            },
        }
    }
}
impl From<ProcessObservationSchema> for ProcessObservation {
    fn from(wire: ProcessObservationSchema) -> Self {
        Self {
            executable_sha256: ArtifactDigest(wire.executable_sha256),
            invocation: wire.invocation.into(),
            stdout: wire.stdout.into(),
            stderr: wire.stderr.into(),
        }
    }
}
impl From<ProcessFailureSchema> for ProcessFailure {
    fn from(wire: ProcessFailureSchema) -> Self {
        let ProcessFailureSchema::ProcessFailure {
            exit,
            stdout,
            stderr,
            observations,
        } = wire;
        Self {
            exit: ProcessExitCode(exit),
            stdout: stdout.into_iter().map(Into::into).collect(),
            stderr: stderr.into_iter().map(Into::into).collect(),
            observations: observations.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum BasisSchema {
    LanguageDefect,
    ToolContractDifference,
}

impl From<BasisSchema> for DiscrepancyBasis {
    fn from(wire: BasisSchema) -> Self {
        match wire {
            BasisSchema::LanguageDefect => Self::LanguageDefect,
            BasisSchema::ToolContractDifference => Self::ToolContract,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct IdentitySchema {
    tool: String,
    version: String,
    observed_executable_sha256: Option<String>,
    profile_sha256: String,
    settings_sha256: String,
    source_sha256: String,
    reference_output_sha256: String,
    intended_output_sha256: String,
    file_metadata_sha256: Option<String>,
}

/// The explicit process failure of a documented difference, if it declares one.
#[derive(Debug, Default, Deserialize)]
#[serde(transparent)]
struct ReferenceProcessSchema(Option<ProcessFailureSchema>);

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DiscrepancySchema {
    basis: Option<BasisSchema>,
    identity: IdentitySchema,
    approved_executable_sha256: Vec<String>,
    reference_expected: Option<Vec<FindingSchema>>,
    intended_expected: Vec<FindingSchema>,
    reason: String,
    #[serde(default)]
    process: ReferenceProcessSchema,
}

impl From<DiscrepancySchema> for DiscrepancyRecord {
    fn from(wire: DiscrepancySchema) -> Self {
        let identity = wire.identity;
        Self {
            basis: wire
                .basis
                .map_or(DiscrepancyBasis::LanguageDefect, Into::into),
            identity: DiscrepancyIdentity {
                tool: ReasonText(identity.tool),
                version: ReasonText(identity.version),
                observed_executable_sha256: sha(identity.observed_executable_sha256)
                    .map_or(ObservedExecutable::NotObserved, ObservedExecutable::Observed),
                profile: Sha256Text(identity.profile_sha256),
                settings: Sha256Text(identity.settings_sha256),
                source: Sha256Text(identity.source_sha256),
                reference_output: Sha256Text(identity.reference_output_sha256),
                intended_output: Sha256Text(identity.intended_output_sha256),
                file_metadata: sha(identity.file_metadata_sha256)
                    .map_or(ryo_benchmark_contracts::domain::documents::types::text::value::FileMetadataDigest::Undeclared, ryo_benchmark_contracts::domain::documents::types::text::value::FileMetadataDigest::Declared),
            },
            approved_executable_sha256: wire
                .approved_executable_sha256
                .into_iter()
                .map(Sha256Text)
                .collect(),
            reference_expected: wire
                .reference_expected
                .map_or(ExpectedFindings::Underivable, |rows| {
                    ExpectedFindings::Derived(findings(rows))
                }),
            intended_expected: findings(wire.intended_expected),
            reason: ReasonText(wire.reason),
            process: match wire.process.0 {
                Some(process) => ryo_benchmark_contracts::domain::documents::types::reference_discrepancy::value::ProcessExpectation::Failure(process.into()),
                None => ryo_benchmark_contracts::domain::documents::types::reference_discrepancy::value::ProcessExpectation::Absent,
            },
        }
    }
}

impl TryFrom<DiscrepancyText<'_>> for DiscrepancyRecord {
    type Error = Error;

    fn try_from(text: DiscrepancyText<'_>) -> Result<Self, Error> {
        serde_json::from_str::<DiscrepancySchema>(text.0)
            .map(Self::from)
            .map_err(|error| Error::Json(Box::new(error)))
    }
}

/// The wire form of one finding.
fn finding_value(finding: &RecordedFinding) -> serde_json::Value {
    serde_json::json!({
        "rule": finding.rule.0,
        "file": finding.file.0,
        "start": finding.span.start.0,
        "end": finding.span.end.0,
    })
}

/// The wire form of every finding, in order.
pub(in super::super) fn findings_value(rows: &[RecordedFinding]) -> serde_json::Value {
    serde_json::Value::Array(rows.iter().map(finding_value).collect())
}

/// The wire form of one output template part.
fn part_value(part: &OutputPart) -> serde_json::Value {
    match part {
        OutputPart::Literal(text) => serde_json::json!({"kind": "literal", "value": text.0}),
        OutputPart::InputPath { file, encoding } => serde_json::json!({
            "kind": "input-path",
            "file": file,
            "encoding": match encoding {
                PathEncoding::Utf8Bytes => "utf8-bytes",
                PathEncoding::JsonStringContent => "json-string-content",
            },
        }),
    }
}

/// The wire form of one raw process artifact.
fn artifact_value(artifact: &ProcessArtifact) -> serde_json::Value {
    serde_json::json!({"path": artifact.path, "sha256": artifact.sha256.0})
}

/// The wire form of an explicit process failure.
fn process_value(process: &ProcessFailure) -> serde_json::Value {
    serde_json::json!({
        "kind": "process-failure",
        "exit": process.exit.0,
        "stdout": process.stdout.iter().map(part_value).collect::<Vec<_>>(),
        "stderr": process.stderr.iter().map(part_value).collect::<Vec<_>>(),
        "observations": process.observations.iter().map(|observation| serde_json::json!({
            "executable-sha256": observation.executable_sha256.0,
            "invocation": artifact_value(&observation.invocation),
            "stdout": artifact_value(&observation.stdout),
            "stderr": artifact_value(&observation.stderr),
        })).collect::<Vec<_>>(),
    })
}

/// The wire form of a documented difference, as the authoring writes it: a process failure carries no basis and
/// no reference findings.
pub(in super::super) fn discrepancy_value(record: &DiscrepancyRecord) -> serde_json::Value {
    let identity = &record.identity;
    let mut written = serde_json::json!({
        "tool": identity.tool.0,
        "version": identity.version.0,
        "profile-sha256": identity.profile.0,
        "settings-sha256": identity.settings.0,
        "source-sha256": identity.source.0,
        "reference-output-sha256": identity.reference_output.0,
        "intended-output-sha256": identity.intended_output.0,
    });
    if let (ObservedExecutable::Observed(digest), Some(fields)) = (
        &identity.observed_executable_sha256,
        written.as_object_mut(),
    ) {
        let _previous = fields.insert("observed-executable-sha256".into(), digest.0.clone().into());
    }
    if let (FileMetadataDigest::Declared(digest), Some(fields)) =
        (&identity.file_metadata, written.as_object_mut())
    {
        let _previous = fields.insert("file-metadata-sha256".into(), digest.0.clone().into());
    }
    let approved = record
        .approved_executable_sha256
        .iter()
        .map(|digest| digest.0.clone())
        .collect::<Vec<_>>();
    match &record.process {
        ProcessExpectation::Failure(process) => serde_json::json!({
            "identity": written,
            "approved_executable_sha256": approved,
            "reference_expected": null,
            "intended_expected": findings_value(&record.intended_expected),
            "reason": record.reason.0,
            "process": process_value(process),
        }),
        ProcessExpectation::Absent => serde_json::json!({
            "basis": match record.basis {
                DiscrepancyBasis::LanguageDefect => "language-defect",
                DiscrepancyBasis::ToolContract => "tool-contract-difference",
            },
            "identity": written,
            "approved_executable_sha256": approved,
            "reference_expected": match &record.reference_expected {
                ExpectedFindings::Derived(rows) => findings_value(rows),
                ExpectedFindings::Underivable => serde_json::Value::Null,
            },
            "intended_expected": findings_value(&record.intended_expected),
            "reason": record.reason.0,
        }),
    }
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
