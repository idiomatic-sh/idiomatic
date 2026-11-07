//! Shared worker wire conversion for explicit performance controls.
use ryo_benchmark_contracts::domain::context::types::rule_observation_declarations::value::RuleObservationDeclarations;
use ryo_benchmark_contracts::domain::context::types::{
    PreparedRuleContext, RuleConfiguration, RuleContextDeclaration,
};
use ryo_benchmark_contracts::domain::requests::types::DiagnosticOptions;
use ryo_benchmark_contracts::domain::requests::types::PerformanceOptions;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct OptionsSchema {
    diagnostics: DiagnosticsSchema,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct DiagnosticsSchema {
    execution: bool,
    lifecycle_memory: bool,
    resource_routes: bool,
    row_progress: bool,
    steps: bool,
    joins: bool,
    prefixes: bool,
    subplans: bool,
    selection_detail: bool,
}

impl From<OptionsSchema> for PerformanceOptions {
    fn from(value: OptionsSchema) -> Self {
        Self {
            diagnostics: DiagnosticOptions {
                execution: value.diagnostics.execution,
                lifecycle_memory: value.diagnostics.lifecycle_memory,
                resource_routes: value.diagnostics.resource_routes,
                row_progress: value.diagnostics.row_progress,
                steps: value.diagnostics.steps,
                joins: value.diagnostics.joins,
                prefixes: value.diagnostics.prefixes,
                subplans: value.diagnostics.subplans,
                selection_detail: value.diagnostics.selection_detail,
            },
        }
    }
}
impl From<PerformanceOptions> for OptionsSchema {
    fn from(value: PerformanceOptions) -> Self {
        Self {
            diagnostics: DiagnosticsSchema {
                execution: value.diagnostics.execution,
                lifecycle_memory: value.diagnostics.lifecycle_memory,
                resource_routes: value.diagnostics.resource_routes,
                row_progress: value.diagnostics.row_progress,
                steps: value.diagnostics.steps,
                joins: value.diagnostics.joins,
                prefixes: value.diagnostics.prefixes,
                subplans: value.diagnostics.subplans,
                selection_detail: value.diagnostics.selection_detail,
            },
        }
    }
}

/// Process-bound spelling; neutral data owns no serde implementation.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "availability", rename_all = "kebab-case", deny_unknown_fields)]
pub(crate) enum ExecutionMetadataSchema {
    Unknown {},
    Unavailable { reason: String },
    Complete {},
}
impl Default for ExecutionMetadataSchema {
    fn default() -> Self {
        Self::Unknown {}
    }
}
impl From<&ryo_benchmark_contracts::domain::reports::types::ExecutionMetadata>
    for ExecutionMetadataSchema
{
    fn from(value: &ryo_benchmark_contracts::domain::reports::types::ExecutionMetadata) -> Self {
        use ryo_benchmark_contracts::domain::reports::types::ExecutionMetadata;
        match value {
            ExecutionMetadata::Unknown => Self::Unknown {},
            ExecutionMetadata::Unavailable { reason } => Self::Unavailable {
                reason: reason.clone(),
            },
            ExecutionMetadata::Complete => Self::Complete {},
        }
    }
}
impl From<ExecutionMetadataSchema>
    for ryo_benchmark_contracts::domain::reports::types::ExecutionMetadata
{
    fn from(value: ExecutionMetadataSchema) -> Self {
        match value {
            ExecutionMetadataSchema::Unknown {} => Self::Unknown,
            ExecutionMetadataSchema::Unavailable { reason } => Self::Unavailable { reason },
            ExecutionMetadataSchema::Complete {} => Self::Complete,
        }
    }
}

/// JSON records the authored data, with no inferred producers or exported diagnostics.
pub(crate) fn observation_declarations_value(
    declarations: &RuleObservationDeclarations,
) -> serde_json::Value {
    use ryo::linter::{ObservationKind, ObservationProducer};
    let entries = declarations
        .iter()
        .map(|(rule, bindings)| {
            let bindings: Vec<_> = bindings.iter().map(|binding| {
            let columns: Vec<_> = binding.columns.iter().map(|column| serde_json::json!({
                "name": column.name.0,
                "kind": match column.kind {
                    ObservationKind::Node => "node",
                    ObservationKind::Symbol => "symbol",
                    ObservationKind::File => "file",
                    ObservationKind::Span => "span",
                    ObservationKind::Number => "number",
                    ObservationKind::Flag => "flag",
                },
            })).collect();
            let producers: Vec<_> = binding.producers.iter().map(|producer| match producer {
                ObservationProducer::Output { rule, output } => serde_json::json!({
                    "kind": "output", "rule": rule.as_ref(), "output": output.0,
                }),
                ObservationProducer::NoQuery { rule } => serde_json::json!({
                    "kind": "no-query", "rule": rule.as_ref(),
                }),
            }).collect();
            serde_json::json!({"name": binding.name.0, "columns": columns, "producers": producers})
        }).collect();
            (rule.to_string(), serde_json::Value::Array(bindings))
        })
        .collect();
    serde_json::Value::Object(entries)
}

/// Retain exactly the authored values in reference/candidate receipt identity.
pub(crate) fn configuration_value(configuration: &RuleConfiguration) -> serde_json::Value {
    let disabled: serde_json::Map<_, _> = configuration
        .disabled_rules
        .iter()
        .map(|(path, rules)| {
            (
                path.0.to_string(),
                serde_json::json!(rules.iter().map(ToString::to_string).collect::<Vec<_>>()),
            )
        })
        .collect();
    serde_json::json!({"external-prefixes": configuration.external_prefixes, "disabled-rules": disabled})
}

/// Profile declarations and per-case data remain distinct identity inputs.
pub(crate) fn declaration_value(declaration: &RuleContextDeclaration) -> serde_json::Value {
    serde_json::json!({
        "tables": {"selected": declaration.tables.selected, "disabled": declaration.tables.disabled, "external": declaration.tables.external},
        "defaults": configuration_value(&declaration.defaults),
    })
}

/// Transport a materialized declaration with the same canonical data encoding used by receipts.
pub(crate) fn prepared_value(prepared: &PreparedRuleContext) -> serde_json::Value {
    serde_json::json!({
        "tables": {"selected": prepared.tables.selected, "disabled": prepared.tables.disabled, "external": prepared.tables.external},
        "configuration": configuration_value(&prepared.configuration),
    })
}

/// The canonical text of the normalized settings a reference process is bound to.
///
/// # Errors
/// Refuses settings the transport encoding cannot spell.
pub(crate) fn invocation_settings_text(
    normalized: &ryo_benchmark_contracts::domain::measurements::types::Invocation,
    case: &ryo_benchmark_contracts::domain::workloads::types::PreparedCase,
    profile: &ryo_benchmark_contracts::domain::workloads::types::Tool,
    observations: Option<&RuleObservationDeclarations>,
) -> Result<String, ryo_benchmark_contracts::domain::reports::errors::ReportsError> {
    use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
    let spell = |value: &std::ffi::OsStr| value.to_string_lossy().into_owned();
    let environment: std::collections::BTreeMap<String, String> = normalized
        .environment
        .iter()
        .map(|(key, value)| (spell(key), spell(value)))
        .collect();
    let mut settings = serde_json::json!({
        "arguments": normalized.arguments.iter().map(|value| spell(value)).collect::<Vec<_>>(),
        "environment": environment,
        "directory": spell(normalized.directory.as_os_str()),
        "parser_configuration": profile.parser_configuration,
        "project_context_mode": format!("{:?}", profile.project_context_mode),
    });
    settings["performance"] = serde_json::to_value(OptionsSchema::from(case.request.performance))
        .map_err(|error| Error::Invalid(error.to_string()))?;
    if let Some(declaration) = &profile.rule_context {
        settings["rule_context_declaration"] = declaration_value(declaration);
    }
    if let Some(context) = &case.request.rule_context {
        settings["rule_context"] = prepared_value(context);
    }
    if let Some(declarations) = observations {
        settings["rule_observations"] = observation_declarations_value(declarations);
    }
    if let Some(selected) = &case.request.selected_rules {
        let selected: std::collections::BTreeSet<_> =
            selected.iter().map(ToString::to_string).collect();
        settings["selected_rules"] = serde_json::json!(selected);
    }
    if let Some(language) = &profile.source_language {
        settings["source_language"] = serde_json::json!(language);
    }
    if let Some(roots) = &case.request.project_search_roots {
        settings["project_search_roots"] =
            serde_json::json!(roots.iter().map(|root| root.0.as_ref()).collect::<Vec<_>>());
    }
    let inventory =
        crate::outbound::context::adapters::local::schemas::ProjectInventorySchema::from(
            case.request.given_project_inventory.clone(),
        );
    if !inventory.is_default() {
        settings["given_project_inventory"] =
            serde_json::to_value(inventory).map_err(|error| Error::Json(Box::new(error)))?;
    }
    Ok(settings.to_string())
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
