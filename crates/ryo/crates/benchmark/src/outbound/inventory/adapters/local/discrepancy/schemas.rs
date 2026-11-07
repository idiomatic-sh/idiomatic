//! The documented difference from the reference tool a case registers, with its findings and its
//! explicit process failure.
use crate::outbound::inventory::adapters::local::types::DiscrepancyText;
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::documents::types::finding::value::{RecordedFinding, FindingOffset};
use ryo_benchmark_contracts::domain::documents::types::finding::value::ExpectedFindings;
use ryo_benchmark_contracts::domain::documents::types::reference_discrepancy::value::{DiscrepancyIdentity, DiscrepancyRecord, ObservedExecutable};
use ryo_benchmark_contracts::domain::documents::types::text::value::{PathText, ReasonText, RuleName, Sha256Text};
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

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
