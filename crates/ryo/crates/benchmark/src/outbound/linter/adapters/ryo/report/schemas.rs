//! The worker report document the child process prints, decoded into the canonical report.
use crate::outbound::linter::adapters::ryo::types::ReportBytes;
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::context::types::ProjectContextReceipt;
use ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::{
    BenchmarkFinding, CoverageAccounting, ExecutionMetadata, PairCount, ProjectContext,
    RuleCoverage, RunReport, SourceByteOffset, UnavailableReason,
};
use serde::Deserialize;
use std::{collections::BTreeMap, path::PathBuf};

/// Candidate findings and exact covered file set.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReportSchema {
    #[serde(default)]
    execution_metadata: ExecutionMetadataSchema,
    #[serde(default)]
    project_context: Option<ReceiptSchema>,
    diagnostics: Vec<DiagnosticSchema>,
    #[serde(default)]
    skipped: Vec<SkippedSchema>,
    #[serde(default)]
    coverage: Option<CoverageSchema>,
    files: Vec<String>,
}
/// Counts of selected pairs by their final outcome.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CoverageSchema {
    selected: usize,
    evaluated: usize,
    skipped: usize,
    unknown_or_unread: usize,
}
/// One typed rule/file skip at the child-process boundary.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SkippedSchema {
    rule: String,
    path: String,
    reason: SkipReasonSchema,
}
/// Why a rule/file pair was not evaluated.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum SkipReasonSchema {
    SyntaxRejected,
}
/// One finding with an unmodified source byte range.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DiagnosticSchema {
    rule: String,
    file: String,
    start: usize,
    end: usize,
}
/// Missing legacy capability remains unknown and is refused for candidate evidence.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(tag = "availability", rename_all = "kebab-case", deny_unknown_fields)]
enum ExecutionMetadataSchema {
    #[default]
    Unknown {},
    Unavailable { reason: String },
    Complete {},
}
impl From<ExecutionMetadataSchema> for ExecutionMetadata {
    fn from(value: ExecutionMetadataSchema) -> Self {
        match value {
            ExecutionMetadataSchema::Unknown {} => Self::Unknown,
            ExecutionMetadataSchema::Unavailable { reason } => Self::Unavailable {
                reason: UnavailableReason(reason),
            },
            ExecutionMetadataSchema::Complete {} => Self::Complete,
        }
    }
}
/// Explicit project lookup mode on the worker wire.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum ModeSchema {
    Isolated,
    ListedDisk,
}
impl From<ModeSchema> for ProjectContextMode {
    fn from(mode: ModeSchema) -> Self {
        match mode {
            ModeSchema::Isolated => Self::Isolated,
            ModeSchema::ListedDisk => Self::ListedDisk,
        }
    }
}
/// Actual dependency observations and the manifest identity that governed them.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReceiptSchema {
    mode: ModeSchema,
    logical_root: PathBuf,
    actual_root: PathBuf,
    manifest_hash: String,
    dependencies: BTreeMap<PathBuf, Option<[u8; 16]>>,
}
impl From<ReceiptSchema> for ProjectContextReceipt {
    fn from(wire: ReceiptSchema) -> Self {
        Self {
            mode: wire.mode.into(),
            logical_root: wire.logical_root,
            actual_root: wire.actual_root,
            manifest_hash: ArtifactDigest(wire.manifest_hash),
            dependencies: wire
                .dependencies
                .into_iter()
                .map(|(path, state)| (path, state.map(ryo::linter::DependencyFingerprint)))
                .collect(),
        }
    }
}

impl TryFrom<ReportSchema> for RunReport {
    type Error = Error;
    fn try_from(wire: ReportSchema) -> Result<Self, Self::Error> {
        let report = Self {
            execution_metadata: wire.execution_metadata.into(),
            project_context: wire
                .project_context
                .map_or(ProjectContext::NotRequested, |receipt| {
                    ProjectContext::Observed(receipt.into())
                }),
            files: wire
                .files
                .into_iter()
                .map(|path| ryo::linter::RelativePath(path.into_boxed_str()))
                .collect(),
            skipped: wire
                .skipped
                .into_iter()
                .map(|skip| {
                    Ok(ryo::linter::Skipped {
                        rule: ryo::linter::RuleId::try_from(skip.rule.as_str())
                            .map_err(|error| Error::Invalid(error.to_string()))?,
                        path: ryo::linter::RelativePath(skip.path.into_boxed_str()),
                        reason: match skip.reason {
                            SkipReasonSchema::SyntaxRejected => {
                                ryo::linter::SkipReason::SyntaxRejected
                            }
                        },
                    })
                })
                .collect::<Result<_, Error>>()?,
            coverage: wire
                .coverage
                .map_or(CoverageAccounting::Unaccounted, |coverage| {
                    CoverageAccounting::Accounted(RuleCoverage {
                        selected: PairCount(coverage.selected),
                        evaluated: PairCount(coverage.evaluated),
                        skipped: PairCount(coverage.skipped),
                        unknown_or_unread: PairCount(coverage.unknown_or_unread),
                    })
                }),
            diagnostics: wire
                .diagnostics
                .into_iter()
                .map(|finding| {
                    Ok(BenchmarkFinding {
                        rule: ryo::linter::RuleId::try_from(finding.rule.as_str())
                            .map_err(|error| Error::Invalid(error.to_string()))?,
                        file: ryo::linter::RelativePath(finding.file.into_boxed_str()),
                        start: SourceByteOffset(finding.start),
                        end: SourceByteOffset(finding.end),
                    })
                })
                .collect::<Result<_, Error>>()?,
        };
        ryo_benchmark_contracts::domain::reports::service::require_complete_execution(&report)?;
        Ok(report)
    }
}
/// Decode candidate findings and validate each rule identifier.
impl TryFrom<ReportBytes<'_>> for RunReport {
    type Error = Error;
    fn try_from(bytes: ReportBytes<'_>) -> Result<Self, Self::Error> {
        let wire: ReportSchema =
            serde_json::from_slice(bytes.0).map_err(|error| Error::Json(Box::new(error)))?;
        wire.try_into()
    }
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
