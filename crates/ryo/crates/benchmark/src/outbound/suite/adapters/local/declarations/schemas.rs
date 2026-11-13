//! The declarations a profile and a cases document share: rule observations, rule context and project scope.
use ryo_benchmark_contracts::domain::context::types::ContextBytes;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::{
    DiagnosticOptions, DiagnosticSwitch, PerformanceOptions,
};

/// Explicit authored input at a private process/TOML serialization boundary.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::outbound::suite::adapters::local) struct ObservationBindingSchema {
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
pub(in crate::outbound::suite::adapters::local) fn observation_declarations(
    declarations: std::collections::BTreeMap<String, Vec<ObservationBindingSchema>>,
) -> Result<
    ryo_benchmark_contracts::domain::context::types::rule_observation_declarations::value::RuleObservationDeclarations,
    String,
>{
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
pub(in crate::outbound::suite::adapters::local) struct RuleConfigurationSchema {
    #[serde(rename = "external-prefixes")]
    external_prefixes: Vec<String>,
    #[serde(rename = "disabled-rules")]
    disabled_rules: std::collections::BTreeMap<String, Vec<String>>,
}
impl From<RuleContextTablesSchema>
    for ryo_benchmark_contracts::domain::context::types::RuleContextTables
{
    fn from(value: RuleContextTablesSchema) -> Self {
        Self {
            selected: ryo_benchmark_contracts::domain::context::types::TableName(value.selected),
            disabled: ryo_benchmark_contracts::domain::context::types::TableName(value.disabled),
            external: ryo_benchmark_contracts::domain::context::types::TableName(value.external),
        }
    }
}
impl TryFrom<RuleConfigurationSchema>
    for ryo_benchmark_contracts::domain::context::types::RuleConfiguration
{
    type Error = String;
    fn try_from(value: RuleConfigurationSchema) -> Result<Self, Self::Error> {
        Ok(Self {
            external_prefixes: value
                .external_prefixes
                .into_iter()
                .map(ryo_benchmark_contracts::domain::context::types::ExternalPrefix)
                .collect(),
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
pub(in crate::outbound::suite::adapters::local) struct RuleContextDeclarationSchema {
    tables: RuleContextTablesSchema,
    defaults: RuleConfigurationSchema,
}
impl TryFrom<RuleContextDeclarationSchema>
    for ryo_benchmark_contracts::domain::context::types::RuleContextDeclaration
{
    type Error = String;
    fn try_from(value: RuleContextDeclarationSchema) -> Result<Self, Self::Error> {
        Ok(Self {
            tables: value.tables.into(),
            defaults: value.defaults.try_into()?,
        })
    }
}
pub(in crate::outbound::suite::adapters::local) fn project_search_roots(
    roots: Option<Vec<String>>,
) -> Result<Option<Vec<ryo::linter::RelativePath>>, Error> {
    roots
        .map(|roots| {
            roots
                .into_iter()
                .map(|root| {
                    if !root.is_empty() {
                        ryo_benchmark_contracts::domain::artifacts::types::relative_artifact_path::value::RelativeArtifactPath::try_from(std::path::Path::new(&root))?;
                    }
                    Ok(ryo::linter::RelativePath(root.into_boxed_str()))
                })
                .collect::<Result<Vec<_>, Error>>()
        })
        .transpose()
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
/// Authored project context materialized independently for one case.
pub(in crate::outbound::suite::adapters::local) struct CaseContextSchema {
    #[serde(rename = "logical-root")]
    /// Stable run-root path inside the case boundary.
    logical_root: std::path::PathBuf,
    #[serde(default, rename = "auxiliary-files")]
    /// Explicit auxiliary file paths and contents.
    auxiliary_files: std::collections::BTreeMap<std::path::PathBuf, ContextBytesSchema>,
    #[serde(default)]
    /// Declared paths that must remain absent.
    missing: Vec<std::path::PathBuf>,
}
#[derive(serde::Deserialize)]
#[serde(untagged)]
/// Text or exact byte content for an auxiliary context file.
enum ContextBytesSchema {
    /// UTF-8 source content.
    Text(String),
    /// Exact bytes, including malformed UTF-8 controls.
    Bytes(Vec<u8>),
}
impl From<CaseContextSchema> for ryo_benchmark_contracts::domain::context::types::CaseContext {
    fn from(value: CaseContextSchema) -> Self {
        Self {
            logical_root: value.logical_root,
            auxiliary_files: value
                .auxiliary_files
                .into_iter()
                .map(|(path, bytes)| {
                    (
                        path,
                        match bytes {
                            ContextBytesSchema::Text(text) => ContextBytes(text.into_bytes()),
                            ContextBytesSchema::Bytes(bytes) => ContextBytes(bytes),
                        },
                    )
                })
                .collect(),
            missing: value.missing,
        }
    }
}

/// How complete a declared project inventory is.
#[derive(Clone, Copy, Debug, Default, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
enum ProjectInventoryCompletenessSchema {
    #[default]
    Partial,
    CompleteWithinDeclaredScope,
}

/// What a declared project entry is.
#[derive(Clone, Copy, Debug, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
enum ProjectEntryStateSchema {
    File,
    Directory,
    Other,
    Missing,
    Unknown,
}

/// One declared project entry.
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectEntrySchema {
    path: String,
    state: ProjectEntryStateSchema,
}

/// The declared project inventory of a case.
#[derive(Clone, Debug, Default, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::outbound::suite::adapters::local) struct ProjectInventorySchema {
    #[serde(default)]
    completeness: ProjectInventoryCompletenessSchema,
    #[serde(default)]
    entries: Vec<ProjectEntrySchema>,
}
impl From<ProjectInventorySchema> for ryo::linter::GivenProjectInventory {
    fn from(wire: ProjectInventorySchema) -> Self {
        Self {
            entries: wire
                .entries
                .into_iter()
                .map(|entry| ryo::linter::ProjectEntryObservation {
                    path: ryo::linter::RelativePath(entry.path.into_boxed_str()),
                    state: match entry.state {
                        ProjectEntryStateSchema::File => ryo::linter::ProjectEntryState::File,
                        ProjectEntryStateSchema::Directory => {
                            ryo::linter::ProjectEntryState::Directory
                        }
                        ProjectEntryStateSchema::Other => ryo::linter::ProjectEntryState::Other,
                        ProjectEntryStateSchema::Missing => ryo::linter::ProjectEntryState::Missing,
                        ProjectEntryStateSchema::Unknown => ryo::linter::ProjectEntryState::Unknown,
                    },
                })
                .collect(),
            completeness: match wire.completeness {
                ProjectInventoryCompletenessSchema::Partial => {
                    ryo::linter::ProjectInventoryCompleteness::Partial
                }
                ProjectInventoryCompletenessSchema::CompleteWithinDeclaredScope => {
                    ryo::linter::ProjectInventoryCompleteness::CompleteWithinDeclaredScope
                }
            },
        }
    }
}

/// The execution and diagnostic policy an authored case selects.
#[derive(Default, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(in crate::outbound::suite::adapters::local) struct OptionsSchema {
    /// Which bounded diagnostic targets the case observes.
    diagnostics: DiagnosticsSchema,
}
/// The bounded diagnostic targets an authored case selects.
#[derive(Default, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the authored spelling is one boolean per bounded diagnostic target"
)]
struct DiagnosticsSchema {
    /// Execution events.
    execution: bool,
    /// Resource route events.
    resource_routes: bool,
    /// Row progress events.
    row_progress: bool,
    /// Step events.
    steps: bool,
    /// Join events.
    joins: bool,
    /// Prefix events.
    prefixes: bool,
    /// Subplan events.
    subplans: bool,
    /// Selection detail events.
    selection_detail: bool,
}
impl From<OptionsSchema> for PerformanceOptions {
    fn from(schema: OptionsSchema) -> Self {
        let switch = |on: bool| {
            if on {
                DiagnosticSwitch::On
            } else {
                DiagnosticSwitch::Off
            }
        };
        let diagnostics = schema.diagnostics;
        Self {
            diagnostics: DiagnosticOptions {
                execution: switch(diagnostics.execution),
                resource_routes: switch(diagnostics.resource_routes),
                row_progress: switch(diagnostics.row_progress),
                steps: switch(diagnostics.steps),
                joins: switch(diagnostics.joins),
                prefixes: switch(diagnostics.prefixes),
                subplans: switch(diagnostics.subplans),
                selection_detail: switch(diagnostics.selection_detail),
            },
        }
    }
}
