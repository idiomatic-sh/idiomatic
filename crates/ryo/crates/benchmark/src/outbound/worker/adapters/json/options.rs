//! Shared worker wire conversion for explicit performance controls.
use crate::domain::suite::types::{DiagnosticOptions, PerformanceOptions};
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
