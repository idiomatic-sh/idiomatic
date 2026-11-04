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
    #[error("invalid selected rule: {0}")]
    InvalidRule(String),
    /// Named-input identities could not be decoded.
    #[error("invalid observation declaration: {0}")]
    Observation(String),
    /// Rule configuration could not be decoded.
    #[error("invalid rule configuration: {0}")]
    Configuration(String),
    /// JSON does not conform to the worker wire representation.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
/// Explicit candidate invocation at the JSON process boundary.
struct RequestSchema {
    #[serde(default)]
    performance: crate::outbound::worker::schemas::OptionsSchema,
    #[serde(default)]
    rule_context: Option<PreparedRuleContextSchema>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    rule_observations: Option<std::collections::BTreeMap<String, Vec<ObservationBindingSchema>>>,
    #[serde(default)]
    selected_rules: Option<Vec<String>>,
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
    /// Explicit module registry carried into the isolated query-loading process.
    #[serde(default)]
    query_modules: std::collections::BTreeMap<String, PathBuf>,
    /// Exact selected-rule parser-applicability policy.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    rule_applicability: std::collections::BTreeMap<String, String>,
    /// Parser configuration identity selected by the frozen profile.
    #[serde(default)]
    parser_configuration: Option<String>,
    #[serde(default)]
    source_language: Option<String>,
    /// Absent means no search authority; an empty array is a declared empty scope.
    #[serde(default)]
    project_search_roots: Option<Vec<String>>,
    #[serde(default)]
    given_project_inventory:
        crate::outbound::context::adapters::local::schemas::ProjectInventorySchema,
    /// Exact project-relative file set covered by this request or report.
    files: Vec<String>,
    /// Retained worker state directory, or no retained state for a fresh run.
    cache: Option<PathBuf>,
}
impl TryFrom<RequestSchema> for WorkerRequest {
    type Error = WorkerSchemaError;
    fn try_from(request: RequestSchema) -> Result<Self, Self::Error> {
        Ok(Self {
            performance: request.performance.into(),
            selected_rules: request
                .selected_rules
                .map(|rules| {
                    rules
                        .into_iter()
                        .map(|id| {
                            ryo::linter::RuleId::try_from(id.as_str())
                                .map_err(|error| WorkerSchemaError::InvalidRule(error.to_string()))
                        })
                        .collect()
                })
                .transpose()?,
            rule_context: request
                .rule_context
                .map(TryInto::try_into)
                .transpose()
                .map_err(WorkerSchemaError::Configuration)?,
            rule_observations: request
                .rule_observations
                .map(observation_declarations)
                .transpose()
                .map_err(WorkerSchemaError::Observation)?,
            project_search_roots: request.project_search_roots.map(|roots| {
                roots
                    .into_iter()
                    .map(|root| ryo::linter::RelativePath(root.into_boxed_str()))
                    .collect()
            }),
            given_project_inventory: request.given_project_inventory.into(),
            project_context_mode: request.project_context_mode.into(),
            context_manifest: request.context_manifest.map(Into::into),
            root: request.root,
            rules: request.rules,
            query_modules: request.query_modules,
            rule_applicability: request.rule_applicability,
            parser_configuration: request.parser_configuration,
            source_language: request.source_language,
            files: request
                .files
                .into_iter()
                .map(|path| ryo::linter::RelativePath(path.into_boxed_str()))
                .collect(),
            state: request.cache.map_or(
                crate::domain::suite::types::WorkerState::Fresh,
                crate::domain::suite::types::WorkerState::Retained,
            ),
        })
    }
}
#[derive(Debug, Serialize)]
/// Candidate findings and exact covered file set.
struct ReportSchema<'a> {
    /// Explicit producer capability; older reports cannot silently acquire completeness.
    execution_metadata: crate::domain::diagnostics::types::ExecutionMetadata,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Actual dependency observations returned by the candidate.
    project_context: Option<crate::outbound::context::adapters::local::schemas::ReceiptSchema>,
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
    /// All requested rule/file pairs.
    selected: usize,
    /// Pairs evaluated to complete candidate outcomes.
    evaluated: usize,
    /// Pairs skipped for explicit parser applicability reasons.
    skipped: usize,
    /// Pairs whose source facts or query outcome remained incomplete.
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
    request.try_into()
}
/// Serialize findings and complete input coverage without deduplication.
pub(super) fn report(report: &RunReport) -> Result<WorkerResponseBytes, WorkerSchemaError> {
    let wire = ReportSchema {
        execution_metadata: report.execution_metadata.clone(),
        project_context: report.project_context.clone().map(Into::into),
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

/// Explicit authored input at a private process/TOML serialization boundary.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ObservationBindingSchema {
    name: String,
    columns: Vec<ObservationColumnSchema>,
    producers: Vec<ObservationProducerSchema>,
}
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ObservationColumnSchema {
    name: String,
    kind: ObservationKindSchema,
}
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
enum ObservationKindSchema {
    Node,
    Symbol,
    File,
    Span,
    Number,
    Flag,
}
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
enum ObservationProducerSchema {
    Output { rule: String, output: String },
    NoQuery { rule: String },
}

/// Convert checked identities while leaving actual producer/schema validation to request compilation.
fn observation_declarations(
    declarations: std::collections::BTreeMap<String, Vec<ObservationBindingSchema>>,
) -> Result<crate::domain::suite::types::RuleObservationDeclarations, String> {
    use ryo::linter::{
        ObservationBinding, ObservationBindingName, ObservationColumn, ObservationColumnName,
        ObservationKind, ObservationName, ObservationProducer, RuleId,
    };
    declarations
        .into_iter()
        .map(|(consumer, bindings)| {
            let consumer =
                RuleId::try_from(consumer.as_str()).map_err(|error| error.to_string())?;
            let bindings = bindings
                .into_iter()
                .map(|binding| {
                    let producers = binding
                        .producers
                        .into_iter()
                        .map(|producer| {
                            Ok(match producer {
                                ObservationProducerSchema::Output { rule, output } => {
                                    ObservationProducer::Output {
                                        rule: RuleId::try_from(rule.as_str())
                                            .map_err(|error| error.to_string())?,
                                        output: ObservationName(output.into()),
                                    }
                                }
                                ObservationProducerSchema::NoQuery { rule } => {
                                    ObservationProducer::NoQuery {
                                        rule: RuleId::try_from(rule.as_str())
                                            .map_err(|error| error.to_string())?,
                                    }
                                }
                            })
                        })
                        .collect::<Result<Vec<_>, String>>()?;
                    Ok(ObservationBinding {
                        name: ObservationBindingName(binding.name.into()),
                        columns: binding
                            .columns
                            .into_iter()
                            .map(|column| ObservationColumn {
                                name: ObservationColumnName(column.name.into()),
                                kind: match column.kind {
                                    ObservationKindSchema::Node => ObservationKind::Node,
                                    ObservationKindSchema::Symbol => ObservationKind::Symbol,
                                    ObservationKindSchema::File => ObservationKind::File,
                                    ObservationKindSchema::Span => ObservationKind::Span,
                                    ObservationKindSchema::Number => ObservationKind::Number,
                                    ObservationKindSchema::Flag => ObservationKind::Flag,
                                },
                            })
                            .collect(),
                        producers,
                    })
                })
                .collect::<Result<Vec<_>, String>>()?;
            Ok((consumer, bindings))
        })
        .collect()
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RuleContextTablesSchema {
    selected: String,
    disabled: String,
    external: String,
}
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RuleConfigurationSchema {
    #[serde(rename = "external-prefixes")]
    external_prefixes: Vec<String>,
    #[serde(rename = "disabled-rules")]
    disabled_rules: std::collections::BTreeMap<String, Vec<String>>,
}
impl From<RuleContextTablesSchema> for crate::domain::suite::types::RuleContextTables {
    fn from(value: RuleContextTablesSchema) -> Self {
        Self {
            selected: value.selected,
            disabled: value.disabled,
            external: value.external,
        }
    }
}
impl TryFrom<RuleConfigurationSchema> for crate::domain::suite::types::RuleConfiguration {
    type Error = String;
    fn try_from(value: RuleConfigurationSchema) -> Result<Self, Self::Error> {
        Ok(Self {
            external_prefixes: value.external_prefixes,
            disabled_rules: value
                .disabled_rules
                .into_iter()
                .map(|(path, rules)| {
                    Ok((
                        ryo::linter::RelativePath(path.into()),
                        rules
                            .into_iter()
                            .map(|rule| {
                                ryo::linter::RuleId::try_from(rule.as_str())
                                    .map_err(|error| error.to_string())
                            })
                            .collect::<Result<Vec<_>, String>>()?,
                    ))
                })
                .collect::<Result<_, String>>()?,
        })
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct PreparedRuleContextSchema {
    tables: RuleContextTablesSchema,
    configuration: RuleConfigurationSchema,
}
impl TryFrom<PreparedRuleContextSchema> for crate::domain::suite::types::PreparedRuleContext {
    type Error = String;
    fn try_from(value: PreparedRuleContextSchema) -> Result<Self, Self::Error> {
        Ok(Self {
            tables: value.tables.into(),
            configuration: value.configuration.try_into()?,
        })
    }
}
