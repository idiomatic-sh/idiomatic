//! JSON exists only at the child-process boundary.
use crate::domain::diagnostics::{
    errors::DiagnosticsError as Error,
    types::{BenchmarkFinding, RunReport},
};
use crate::domain::suite::types::WorkerRequest;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Explicit candidate invocation at the JSON process boundary.
struct RyoWorkerRequestSchema {
    /// Project root against which explicit file paths are resolved.
    root: PathBuf,
    /// Directory containing the selected RyoQL rule definitions.
    rules: PathBuf,
    /// Explicit logical module registry under the frozen profile.
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
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Candidate findings and exact covered file set.
struct RyoReportSchema {
    /// Reported findings, preserving multiplicity and source offsets.
    diagnostics: Vec<RyoDiagnosticSchema>,
    /// Rules explicitly skipped after typed syntax rejection.
    #[serde(default)]
    skipped: Vec<RyoSkippedSchema>,
    /// Rule/file-pair accounting returned by the candidate worker.
    #[serde(default)]
    coverage: Option<RyoCoverageSchema>,
    /// Exact project-relative file set covered by this request or report.
    files: Vec<String>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Counts of selected pairs by their final outcome.
struct RyoCoverageSchema {
    selected: usize,
    evaluated: usize,
    skipped: usize,
    unknown_or_unread: usize,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// One typed rule/file skip at the child-process boundary.
struct RyoSkippedSchema {
    /// Selected rule that was not evaluated.
    rule: String,
    /// Exact input file for which syntax was rejected.
    path: String,
    /// Stable machine-readable reason.
    reason: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// One finding with an unmodified source byte range.
struct RyoDiagnosticSchema {
    /// Stable rule identifier for this finding.
    rule: String,
    /// Project-relative source path for this finding.
    file: String,
    /// Inclusive zero-based UTF-8 byte offset.
    start: usize,
    /// Exclusive zero-based UTF-8 byte offset.
    end: usize,
}

impl From<WorkerRequest> for RyoWorkerRequestSchema {
    fn from(request: WorkerRequest) -> Self {
        Self {
            root: request.root,
            rules: request.rules,
            query_modules: request.query_modules,
            rule_applicability: request.rule_applicability,
            parser_configuration: request.parser_configuration,
            files: request
                .files
                .into_iter()
                .map(|path| path.0.into_string())
                .collect(),
            cache: match request.state {
                crate::domain::suite::types::WorkerState::Fresh => None,
                crate::domain::suite::types::WorkerState::Retained(path) => Some(path),
            },
        }
    }
}
impl TryFrom<RyoReportSchema> for RunReport {
    type Error = Error;
    fn try_from(wire: RyoReportSchema) -> Result<Self, Self::Error> {
        Ok(Self {
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
                        reason: match skip.reason.as_str() {
                            "syntax-rejected" => ryo::linter::SkipReason::SyntaxRejected,
                            other => {
                                return Err(Error::Invalid(format!(
                                    "unsupported skip reason: {other}"
                                )));
                            }
                        },
                    })
                })
                .collect::<Result<_, Error>>()?,
            coverage: wire.coverage.map(|coverage| {
                crate::domain::diagnostics::types::RuleCoverage {
                    selected: coverage.selected,
                    evaluated: coverage.evaluated,
                    skipped: coverage.skipped,
                    unknown_or_unread: coverage.unknown_or_unread,
                }
            }),
            diagnostics: wire
                .diagnostics
                .into_iter()
                .map(|finding| {
                    Ok(BenchmarkFinding {
                        rule: ryo::linter::RuleId::try_from(finding.rule.as_str())
                            .map_err(|error| Error::Invalid(error.to_string()))?,
                        file: ryo::linter::RelativePath(finding.file.into_boxed_str()),
                        start: crate::domain::diagnostics::types::SourceByteOffset(finding.start),
                        end: crate::domain::diagnostics::types::SourceByteOffset(finding.end),
                    })
                })
                .collect::<Result<_, Error>>()?,
        })
    }
}
/// Encode a typed candidate invocation for the child process.
pub(super) fn encode_request(request: WorkerRequest) -> Result<String, Error> {
    serde_json::to_string(&RyoWorkerRequestSchema::from(request))
        .map_err(|error| Error::Json(Box::new(error)))
}
/// Decode candidate findings and validate each rule identifier.
pub(super) fn decode_report(bytes: &[u8]) -> Result<RunReport, Error> {
    let wire: RyoReportSchema =
        serde_json::from_slice(bytes).map_err(|error| Error::Json(Box::new(error)))?;
    wire.try_into()
}
#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
