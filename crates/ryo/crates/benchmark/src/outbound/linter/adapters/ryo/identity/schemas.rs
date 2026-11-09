//! The canonical identity texts a reference process is bound to, spelled exactly as receipts compare them.
use crate::outbound::linter::types::CanonicalText;
use ryo::linter::{ObservationKind, ObservationProducer};
use ryo_benchmark_contracts::domain::context::types::rule_observation_declarations::value::RuleObservationDeclarations;
use ryo_benchmark_contracts::domain::context::types::{
    PreparedRuleContext, RuleConfiguration, RuleContextDeclaration,
};
use ryo_benchmark_contracts::domain::measurements::types::Invocation;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::{
    CohortSelection, DiagnosticSwitch, InputDeclaration, ParserSelection, PerformanceOptions,
    ProviderSelection, RuleContextPreparation, SearchRoots,
};
use ryo_benchmark_contracts::domain::workloads::types::RuleContextProfile;
use ryo_benchmark_contracts::domain::workloads::types::{PreparedCase, Tool};
use serde::Serialize;

/// Explicit performance controls on the worker wire.
#[derive(Clone, Copy, Debug, Default, Serialize)]
#[serde(default, deny_unknown_fields)]
struct OptionsSchema {
    diagnostics: DiagnosticsSchema,
}
#[derive(Clone, Copy, Debug, Default, Serialize)]
#[serde(default, deny_unknown_fields)]
struct DiagnosticsSchema {
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
impl From<PerformanceOptions> for OptionsSchema {
    fn from(value: PerformanceOptions) -> Self {
        let on = |switch: DiagnosticSwitch| matches!(switch, DiagnosticSwitch::On);
        let diagnostics = value.diagnostics;
        Self {
            diagnostics: DiagnosticsSchema {
                execution: on(diagnostics.execution),
                lifecycle_memory: on(diagnostics.lifecycle_memory),
                resource_routes: on(diagnostics.resource_routes),
                row_progress: on(diagnostics.row_progress),
                steps: on(diagnostics.steps),
                joins: on(diagnostics.joins),
                prefixes: on(diagnostics.prefixes),
                subplans: on(diagnostics.subplans),
                selection_detail: on(diagnostics.selection_detail),
            },
        }
    }
}

/// How complete the declared project inventory is.
#[derive(Clone, Copy, Debug, Default, Serialize)]
#[serde(rename_all = "kebab-case")]
enum ProjectInventoryCompletenessSchema {
    #[default]
    Partial,
    CompleteWithinDeclaredScope,
}
/// What a declared project entry is.
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
enum ProjectEntryStateSchema {
    File,
    Directory,
    Other,
    Missing,
    Unknown,
}
/// One declared project entry.
#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
struct ProjectEntrySchema {
    path: String,
    state: ProjectEntryStateSchema,
}
/// The declared project inventory of a request.
#[derive(Clone, Debug, Default, Serialize)]
#[serde(deny_unknown_fields)]
struct ProjectInventorySchema {
    completeness: ProjectInventoryCompletenessSchema,
    entries: Vec<ProjectEntrySchema>,
}
impl ProjectInventorySchema {
    fn is_default(&self) -> bool {
        matches!(
            self.completeness,
            ProjectInventoryCompletenessSchema::Partial
        ) && self.entries.is_empty()
    }
}
impl From<ryo::linter::GivenProjectInventory> for ProjectInventorySchema {
    fn from(value: ryo::linter::GivenProjectInventory) -> Self {
        Self {
            entries: value
                .entries
                .into_iter()
                .map(|entry| ProjectEntrySchema {
                    path: entry.path.0.into_string(),
                    state: match entry.state {
                        ryo::linter::ProjectEntryState::File => ProjectEntryStateSchema::File,
                        ryo::linter::ProjectEntryState::Directory => {
                            ProjectEntryStateSchema::Directory
                        }
                        ryo::linter::ProjectEntryState::Other => ProjectEntryStateSchema::Other,
                        ryo::linter::ProjectEntryState::Missing => ProjectEntryStateSchema::Missing,
                        ryo::linter::ProjectEntryState::Unknown => ProjectEntryStateSchema::Unknown,
                    },
                })
                .collect(),
            completeness: match value.completeness {
                ryo::linter::ProjectInventoryCompleteness::Partial => {
                    ProjectInventoryCompletenessSchema::Partial
                }
                ryo::linter::ProjectInventoryCompleteness::CompleteWithinDeclaredScope => {
                    ProjectInventoryCompletenessSchema::CompleteWithinDeclaredScope
                }
            },
        }
    }
}

/// JSON records the authored data, with no inferred producers or exported diagnostics.
pub(in crate::outbound::linter::adapters::ryo) fn observation_declarations_value(
    declarations: &ryo_benchmark_contracts::domain::context::types::rule_observation_declarations::value::RuleObservationDeclarations,
) -> serde_json::Value {
    let entries = declarations
        .iter()
        .map(|(rule, bindings)| {
            let bindings: Vec<_> = bindings
                .iter()
                .map(|binding| {
                    let columns: Vec<_> = binding
                        .columns
                        .iter()
                        .map(|column| {
                            serde_json::json!({
                                "name": column.name.0,
                                "kind": match column.kind {
                                    ObservationKind::Node => "node",
                                    ObservationKind::Symbol => "symbol",
                                    ObservationKind::File => "file",
                                    ObservationKind::Span => "span",
                                    ObservationKind::Number => "number",
                                    ObservationKind::Flag => "flag",
                                },
                            })
                        })
                        .collect();
                    let producers: Vec<_> = binding
                        .producers
                        .iter()
                        .map(|producer| match producer {
                            ObservationProducer::Output { rule, output } => serde_json::json!({
                                "kind": "output", "rule": rule.as_ref(), "output": output.0,
                            }),
                            ObservationProducer::NoQuery { rule } => serde_json::json!({
                                "kind": "no-query", "rule": rule.as_ref(),
                            }),
                        })
                        .collect();
                    serde_json::json!({"name": binding.name.0, "columns": columns, "producers": producers})
                })
                .collect();
            (rule.to_string(), serde_json::Value::Array(bindings))
        })
        .collect();
    serde_json::Value::Object(entries)
}

/// Retain exactly the authored values in reference/candidate receipt identity.
fn configuration_value(configuration: &RuleConfiguration) -> serde_json::Value {
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
    let prefixes: Vec<&str> = configuration
        .external_prefixes
        .iter()
        .map(|prefix| prefix.0.as_str())
        .collect();
    serde_json::json!({"external-prefixes": prefixes, "disabled-rules": disabled})
}

/// Profile declarations and per-case data remain distinct identity inputs.
fn declaration_value(declaration: &RuleContextDeclaration) -> serde_json::Value {
    serde_json::json!({
        "tables": {"selected": declaration.tables.selected.0, "disabled": declaration.tables.disabled.0, "external": declaration.tables.external.0},
        "defaults": configuration_value(&declaration.defaults),
    })
}

/// Transport a materialized declaration with the same canonical data encoding used by receipts.
pub(in crate::outbound::linter::adapters::ryo) fn prepared_value(
    prepared: &PreparedRuleContext,
) -> serde_json::Value {
    serde_json::json!({
        "tables": {"selected": prepared.tables.selected.0, "disabled": prepared.tables.disabled.0, "external": prepared.tables.external.0},
        "configuration": configuration_value(&prepared.configuration),
    })
}

/// The canonical text of a materialized rule configuration, as receipts compare it.
impl From<&PreparedRuleContext> for CanonicalText {
    fn from(prepared: &PreparedRuleContext) -> Self {
        Self(prepared_value(prepared).to_string())
    }
}
/// The canonical text of authored observation declarations, as receipts carry them.
impl From<&RuleObservationDeclarations> for CanonicalText {
    fn from(declarations: &RuleObservationDeclarations) -> Self {
        Self(observation_declarations_value(declarations).to_string())
    }
}

/// The rule observations an invocation is bound to: the case's own, else the profile's projection.
fn effective_observations(
    case: &PreparedCase,
    profile: &Tool,
) -> Option<RuleObservationDeclarations> {
    if let InputDeclaration::Declared(declarations) = &case.request.rule_observations {
        return Some(declarations.clone());
    }
    if profile.rule_observations.is_empty() {
        return None;
    }
    let declarations = profile.rule_observations.project_profile(
        match &case.request.selected_rules {
            CohortSelection::Explicit(selected) => selected,
            CohortSelection::Frozen => &profile.defaults,
        },
        profile.rule_observation_selection,
    );
    (!declarations.is_empty()).then_some(declarations)
}

/// The canonical text of the normalized settings a reference process is bound to: the invocation
/// after normalization, the prepared case it runs and the profile the case belongs to.
impl TryFrom<(&Invocation, &PreparedCase, &Tool)> for CanonicalText {
    type Error = Error;
    fn try_from(
        (normalized, case, profile): (&Invocation, &PreparedCase, &Tool),
    ) -> Result<Self, Self::Error> {
        let observations = effective_observations(case, profile);
        let spell = |value: &std::ffi::OsStr| value.to_string_lossy().into_owned();
        let environment: std::collections::BTreeMap<String, String> = normalized
            .environment
            .iter()
            .map(|(key, value)| (spell(key), spell(value)))
            .collect();
        let parser_configuration = match &profile.parser_configuration {
            ParserSelection::Unspecified => serde_json::Value::Null,
            ParserSelection::Named(name) => serde_json::json!(name.0),
        };
        let mut settings = serde_json::json!({
            "arguments": normalized.arguments.iter().map(|value| spell(value)).collect::<Vec<_>>(),
            "environment": environment,
            "directory": spell(normalized.directory.as_os_str()),
            "parser_configuration": parser_configuration,
            "project_context_mode": format!("{:?}", profile.project_context_mode),
        });
        settings["performance"] =
            serde_json::to_value(OptionsSchema::from(case.request.performance))
                .map_err(|error| Error::Invalid(error.to_string()))?;
        if let RuleContextProfile::Declared(declaration) = &profile.rule_context {
            settings["rule_context_declaration"] = declaration_value(declaration);
        }
        if let RuleContextPreparation::Prepared(context) = &case.request.rule_context {
            settings["rule_context"] = prepared_value(context);
        }
        if let Some(declarations) = &observations {
            settings["rule_observations"] = observation_declarations_value(declarations);
        }
        if let CohortSelection::Explicit(selected) = &case.request.selected_rules {
            let selected: std::collections::BTreeSet<_> =
                selected.iter().map(ToString::to_string).collect();
            settings["selected_rules"] = serde_json::json!(selected);
        }
        if let ProviderSelection::Selected(language) = &profile.source_language {
            settings["source_language"] = serde_json::json!(language.0);
        }
        if let SearchRoots::Declared(roots) = &case.request.project_search_roots {
            settings["project_search_roots"] =
                serde_json::json!(roots.iter().map(|root| root.0.as_ref()).collect::<Vec<_>>());
        }
        let inventory = ProjectInventorySchema::from(case.request.given_project_inventory.clone());
        if !inventory.is_default() {
            settings["given_project_inventory"] =
                serde_json::to_value(inventory).map_err(|error| Error::Json(Box::new(error)))?;
        }
        Ok(Self(settings.to_string()))
    }
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
