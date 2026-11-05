//! JSON exists only at the child-process boundary.
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::BenchmarkFinding;
use ryo_benchmark_contracts::domain::reports::types::RunReport;
use ryo_benchmark_contracts::domain::requests::types::WorkerRequest;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
/// Explicit candidate invocation at the JSON process boundary.
struct RyoWorkerRequestSchema {
    #[serde(default)]
    performance: crate::outbound::worker::schemas::OptionsSchema,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    selected_rules: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    rule_context: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    rule_observations: Option<serde_json::Value>,
    #[serde(default)]
    /// Explicit project lookup mode for this invocation.
    project_context_mode: crate::outbound::context::adapters::local::schemas::ModeSchema,
    #[serde(default)]
    /// Sealed source, auxiliary and missing context inventory.
    context_manifest: Option<crate::outbound::context::adapters::local::schemas::ManifestSchema>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    source_language: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    project_search_roots: Option<Vec<String>>,
    #[serde(
        default,
        skip_serializing_if = "crate::outbound::context::adapters::local::schemas::ProjectInventorySchema::is_default"
    )]
    given_project_inventory:
        crate::outbound::context::adapters::local::schemas::ProjectInventorySchema,
    /// Exact project-relative file set covered by this request or report.
    files: Vec<String>,
    /// Retained worker state directory, or no retained state for a fresh run.
    cache: Option<PathBuf>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Candidate findings and exact covered file set.
struct RyoReportSchema {
    /// Missing legacy capability remains unknown and is refused for candidate evidence.
    #[serde(default)]
    execution_metadata: crate::outbound::worker::schemas::ExecutionMetadataSchema,
    #[serde(default)]
    /// Actual dependency observations returned by the candidate.
    project_context: Option<crate::outbound::context::adapters::local::schemas::ReceiptSchema>,
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
    /// All requested rule/file pairs.
    selected: usize,
    /// Pairs evaluated to complete candidate outcomes.
    evaluated: usize,
    /// Pairs skipped for explicit parser applicability reasons.
    skipped: usize,
    /// Pairs whose source facts or query outcome remained incomplete.
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
            performance: request.performance.into(),
            rule_context: request
                .rule_context
                .as_ref()
                .map(crate::outbound::worker::schemas::prepared_value),
            rule_observations: request
                .rule_observations
                .as_ref()
                .map(crate::outbound::worker::schemas::observation_declarations_value),
            project_context_mode: request.project_context_mode.into(),
            context_manifest: request.context_manifest.map(Into::into),
            root: request.root,
            rules: request.rules,
            selected_rules: request
                .selected_rules
                .map(|rules| rules.into_iter().map(|id| id.to_string()).collect()),
            query_modules: request.query_modules,
            rule_applicability: request.rule_applicability,
            parser_configuration: request.parser_configuration,
            source_language: request.source_language,
            project_search_roots: request
                .project_search_roots
                .map(|roots| roots.into_iter().map(|root| root.0.into_string()).collect()),
            given_project_inventory: request.given_project_inventory.into(),
            files: request
                .files
                .into_iter()
                .map(|path| path.0.into_string())
                .collect(),
            cache: match request.state {
                ryo_benchmark_contracts::domain::requests::types::WorkerState::Fresh => None,
                ryo_benchmark_contracts::domain::requests::types::WorkerState::Retained(path) => {
                    Some(path)
                }
            },
        }
    }
}
impl TryFrom<RyoReportSchema> for RunReport {
    type Error = Error;
    fn try_from(wire: RyoReportSchema) -> Result<Self, Self::Error> {
        let report = Self {
            execution_metadata: wire.execution_metadata.into(),
            project_context: wire.project_context.map(Into::into),
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
                ryo_benchmark_contracts::domain::reports::types::RuleCoverage {
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
                        start: ryo_benchmark_contracts::domain::reports::types::SourceByteOffset(
                            finding.start,
                        ),
                        end: ryo_benchmark_contracts::domain::reports::types::SourceByteOffset(
                            finding.end,
                        ),
                    })
                })
                .collect::<Result<_, Error>>()?,
        };
        crate::domain::diagnostics::service::require_complete_execution(&report)
            .map_err(|error| Error::Invalid(error.into()))?;
        Ok(report)
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
