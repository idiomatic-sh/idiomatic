//! Shared worker wire conversion for explicit performance controls.
use ryo_benchmark_contracts::domain::context::types::rule_observation_declarations::value::RuleObservationDeclarations;
use ryo_benchmark_contracts::domain::context::types::{
    PreparedRuleContext, RuleConfiguration, RuleContextDeclaration,
};
use ryo_benchmark_contracts::domain::requests::types::DiagnosticOptions;
use ryo_benchmark_contracts::domain::requests::types::PerformanceOptions;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum NodeReuseSchema {
    #[default]
    Disabled,
    Enabled,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct OptionsSchema {
    node_result_reuse: NodeReuseSchema,
    column_kernel: bool,
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
            execution: ryo::linter::ExecutionOptions {
                node_result_reuse: match value.node_result_reuse {
                    NodeReuseSchema::Disabled => ryo::linter::NodeResultReuse::Disabled,
                    NodeReuseSchema::Enabled => ryo::linter::NodeResultReuse::Enabled,
                },
                column_kernel: value.column_kernel,
            },
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
            node_result_reuse: match value.execution.node_result_reuse {
                ryo::linter::NodeResultReuse::Disabled => NodeReuseSchema::Disabled,
                ryo::linter::NodeResultReuse::Enabled => NodeReuseSchema::Enabled,
            },
            column_kernel: value.execution.column_kernel,
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
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(tag = "availability", rename_all = "kebab-case", deny_unknown_fields)]
pub(crate) enum ExecutionMetadataSchema {
    #[default]
    Unknown,
    Unavailable {
        reason: String,
    },
    Complete,
}
impl From<&ryo_benchmark_contracts::domain::reports::types::ExecutionMetadata>
    for ExecutionMetadataSchema
{
    fn from(value: &ryo_benchmark_contracts::domain::reports::types::ExecutionMetadata) -> Self {
        use ryo_benchmark_contracts::domain::reports::types::ExecutionMetadata;
        match value {
            ExecutionMetadata::Unknown => Self::Unknown,
            ExecutionMetadata::Unavailable { reason } => Self::Unavailable {
                reason: reason.clone(),
            },
            ExecutionMetadata::Complete => Self::Complete,
        }
    }
}
impl From<ExecutionMetadataSchema>
    for ryo_benchmark_contracts::domain::reports::types::ExecutionMetadata
{
    fn from(value: ExecutionMetadataSchema) -> Self {
        match value {
            ExecutionMetadataSchema::Unknown => Self::Unknown,
            ExecutionMetadataSchema::Unavailable { reason } => Self::Unavailable { reason },
            ExecutionMetadataSchema::Complete => Self::Complete,
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

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
