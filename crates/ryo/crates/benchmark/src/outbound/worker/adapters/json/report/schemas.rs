//! The worker report document: findings and complete input coverage, as the parent process reads it.
use crate::outbound::worker::adapters::json::types::WorkerResponseBytes;
use ryo_benchmark_contracts::domain::context::types::ProjectContextReceipt;
use ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::{
    CoverageAccounting, ExecutionMetadata, ProjectContext, RunReport, UnavailableReason,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};

/// Candidate findings and exact covered file set.
#[derive(Debug, Serialize)]
struct ReportSchema<'a> {
    execution_metadata: ExecutionMetadataSchema,
    #[serde(skip_serializing_if = "Option::is_none")]
    project_context: Option<ReceiptSchema>,
    diagnostics: Vec<DiagnosticSchema<'a>>,
    skipped: Vec<SkippedSchema<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    coverage: Option<CoverageSchema>,
    files: Vec<&'a str>,
}
/// Counts of selected rule/file pairs by final outcome.
#[derive(Debug, Serialize)]
struct CoverageSchema {
    selected: usize,
    evaluated: usize,
    skipped: usize,
    unknown_or_unread: usize,
}
/// One selected rule/file pair not evaluated after explicit syntax rejection.
#[derive(Debug, Serialize)]
struct SkippedSchema<'a> {
    rule: &'a str,
    path: &'a str,
    reason: SkipReasonSchema,
}
/// Why a rule/file pair was not evaluated.
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
enum SkipReasonSchema {
    SyntaxRejected,
}
/// One finding with an unmodified source byte range.
#[derive(Debug, Serialize)]
struct DiagnosticSchema<'a> {
    rule: &'a str,
    file: &'a str,
    start: usize,
    end: usize,
}

/// Process-bound spelling; neutral data owns no serde implementation.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "availability", rename_all = "kebab-case", deny_unknown_fields)]
enum ExecutionMetadataSchema {
    Unknown {},
    Unavailable { reason: String },
    Complete {},
}
impl From<&ExecutionMetadata> for ExecutionMetadataSchema {
    fn from(value: &ExecutionMetadata) -> Self {
        match value {
            ExecutionMetadata::Unknown => Self::Unknown {},
            ExecutionMetadata::Unavailable { reason } => Self::Unavailable {
                reason: reason.0.clone(),
            },
            ExecutionMetadata::Complete => Self::Complete {},
        }
    }
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

/// Explicit project lookup mode of a worker report.
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
enum ModeSchema {
    Isolated,
    ListedDisk,
}
impl From<ProjectContextMode> for ModeSchema {
    fn from(mode: ProjectContextMode) -> Self {
        match mode {
            ProjectContextMode::Isolated => Self::Isolated,
            ProjectContextMode::ListedDisk => Self::ListedDisk,
        }
    }
}
/// The actual dependency observations a worker report carries.
#[derive(Clone, Debug, Serialize)]
struct ReceiptSchema {
    mode: ModeSchema,
    logical_root: PathBuf,
    actual_root: PathBuf,
    manifest_hash: String,
    dependencies: BTreeMap<PathBuf, Option<[u8; 16]>>,
}
impl From<ProjectContextReceipt> for ReceiptSchema {
    fn from(value: ProjectContextReceipt) -> Self {
        Self {
            mode: value.mode.into(),
            logical_root: value.logical_root,
            actual_root: value.actual_root,
            manifest_hash: value.manifest_hash.0,
            dependencies: value
                .dependencies
                .into_iter()
                .map(|(path, state)| (path, state.map(|fingerprint| fingerprint.0)))
                .collect(),
        }
    }
}

/// Serialize findings and complete input coverage without deduplication.
impl TryFrom<&RunReport> for WorkerResponseBytes {
    type Error = Error;
    fn try_from(report: &RunReport) -> Result<Self, Self::Error> {
        let wire = ReportSchema {
            execution_metadata: (&report.execution_metadata).into(),
            project_context: match &report.project_context {
                ProjectContext::NotRequested => None,
                ProjectContext::Observed(receipt) => Some(receipt.clone().into()),
            },
            files: report.files.iter().map(|path| path.0.as_ref()).collect(),
            diagnostics: report
                .diagnostics
                .iter()
                .map(|finding| DiagnosticSchema {
                    rule: finding.rule.as_ref(),
                    file: finding.file.0.as_ref(),
                    start: finding.start.0,
                    end: finding.end.0,
                })
                .collect(),
            skipped: report
                .skipped
                .iter()
                .map(|skip| SkippedSchema {
                    rule: skip.rule.as_ref(),
                    path: skip.path.0.as_ref(),
                    reason: match skip.reason {
                        ryo::linter::SkipReason::SyntaxRejected => SkipReasonSchema::SyntaxRejected,
                    },
                })
                .collect(),
            coverage: match report.coverage {
                CoverageAccounting::Unaccounted => None,
                CoverageAccounting::Accounted(coverage) => Some(CoverageSchema {
                    selected: coverage.selected.0,
                    evaluated: coverage.evaluated.0,
                    skipped: coverage.skipped.0,
                    unknown_or_unread: coverage.unknown_or_unread.0,
                }),
            },
        };
        Ok(Self(
            serde_json::to_vec(&wire).map_err(|error| Error::Json(Box::new(error)))?,
        ))
    }
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
