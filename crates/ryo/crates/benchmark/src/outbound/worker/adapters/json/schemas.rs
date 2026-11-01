//! Private wire shapes; neither canonical values nor callers depend on serde.
use super::types::WorkerResponseBytes;
use crate::domain::diagnostics::types::RunReport;
use crate::domain::suite::types::WorkerRequest;
use serde::{Deserialize, Serialize};
use std::{ffi::OsStr, path::PathBuf};

#[derive(Debug, thiserror::Error)]
/// Request decoding or response encoding failure at the worker boundary.
pub(super) enum WorkerSchemaError {
    /// Request argument cannot be interpreted as JSON text.
    #[error("worker request is not UTF-8")]
    NotText,
    /// JSON does not conform to the worker wire representation.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
/// Explicit candidate invocation at the JSON process boundary.
struct RequestSchema {
    /// Project root against which explicit file paths are resolved.
    root: PathBuf,
    /// Directory containing the selected RyoQL rule definitions.
    rules: PathBuf,
    /// Explicit module registry carried into the isolated query-loading process.
    #[serde(default)]
    query_modules: std::collections::BTreeMap<String, PathBuf>,
    /// Exact selected-rule parser-applicability policy.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    rule_applicability: std::collections::BTreeMap<String, String>,
    /// Parser configuration identity selected by the frozen profile.
    #[serde(default)]
    parser_configuration: Option<String>,
    /// Exact project-relative file set covered by this request or report.
    files: Vec<String>,
    /// Retained worker state directory, or no retained state for a fresh run.
    cache: Option<PathBuf>,
}
impl From<RequestSchema> for WorkerRequest {
    fn from(request: RequestSchema) -> Self {
        Self {
            root: request.root,
            rules: request.rules,
            query_modules: request.query_modules,
            rule_applicability: request.rule_applicability,
            parser_configuration: request.parser_configuration,
            files: request
                .files
                .into_iter()
                .map(|path| ryo::linter::RelativePath(path.into_boxed_str()))
                .collect(),
            state: request.cache.map_or(
                crate::domain::suite::types::WorkerState::Fresh,
                crate::domain::suite::types::WorkerState::Retained,
            ),
        }
    }
}
#[derive(Debug, Serialize)]
/// Candidate findings and exact covered file set.
struct ReportSchema<'a> {
    /// Reported findings, preserving multiplicity and source offsets.
    diagnostics: Vec<DiagnosticSchema<'a>>,
    /// Typed syntax-applicability skips, kept distinct from empty results.
    skipped: Vec<SkippedSchema<'a>>,
    /// Exact selected/evaluated/skipped accounting for a candidate run.
    #[serde(skip_serializing_if = "Option::is_none")]
    coverage: Option<CoverageSchema>,
    /// Exact project-relative file set covered by this request or report.
    files: Vec<&'a str>,
}
#[derive(Debug, Serialize)]
/// Counts of selected rule/file pairs by final outcome.
struct CoverageSchema {
    selected: usize,
    evaluated: usize,
    skipped: usize,
    unknown_or_unread: usize,
}
#[derive(Debug, Serialize)]
/// One selected rule/file pair not evaluated after explicit syntax rejection.
struct SkippedSchema<'a> {
    /// Stable selected rule identifier.
    rule: &'a str,
    /// Exact project-relative file path.
    path: &'a str,
    /// Stable machine-readable skip reason.
    reason: &'static str,
}
#[derive(Debug, Serialize)]
/// One finding with an unmodified source byte range.
struct DiagnosticSchema<'a> {
    /// Stable rule identifier for this finding.
    rule: &'a str,
    /// Project-relative source path for this finding.
    file: &'a str,
    /// Inclusive zero-based UTF-8 byte offset.
    start: usize,
    /// Exclusive zero-based UTF-8 byte offset.
    end: usize,
}
/// Decode the explicit worker invocation from its command-line payload.
pub(super) fn request(payload: &OsStr) -> Result<WorkerRequest, WorkerSchemaError> {
    let text = payload.to_str().ok_or(WorkerSchemaError::NotText)?;
    let request: RequestSchema = serde_json::from_str(text)?;
    Ok(request.into())
}
/// Serialize findings and complete input coverage without deduplication.
pub(super) fn report(report: &RunReport) -> Result<WorkerResponseBytes, WorkerSchemaError> {
    let wire = ReportSchema {
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
                    ryo::linter::SkipReason::SyntaxRejected => "syntax-rejected",
                },
            })
            .collect(),
        coverage: report.coverage.map(|coverage| CoverageSchema {
            selected: coverage.selected,
            evaluated: coverage.evaluated,
            skipped: coverage.skipped,
            unknown_or_unread: coverage.unknown_or_unread,
        }),
    };
    Ok(WorkerResponseBytes(serde_json::to_vec(&wire)?))
}
#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
