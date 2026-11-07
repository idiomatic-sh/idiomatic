//! The worker request document the parent process writes for its child.
use crate::outbound::linter::adapters::ryo::types::RequestJson;
use crate::outbound::linter::types::{ObservationsInput, RuleContextInput};
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::context::types::ContextManifest;
use ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::{
    CohortSelection, ContextSeal, DiagnosticSwitch, InputDeclaration, ParserSelection,
    PerformanceOptions, ProviderSelection, RuleContextPreparation, SearchRoots, WorkerRequest,
    WorkerState,
};
use serde::Serialize;
use std::{collections::BTreeMap, path::PathBuf};

/// Explicit candidate invocation at the JSON process boundary.
#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct RequestSchema {
    performance: OptionsSchema,
    #[serde(skip_serializing_if = "Option::is_none")]
    selected_rules: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rule_context: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rule_observations: Option<serde_json::Value>,
    project_context_mode: ModeSchema,
    context_manifest: Option<ManifestSchema>,
    root: PathBuf,
    rules: PathBuf,
    query_modules: BTreeMap<String, PathBuf>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    rule_applicability: BTreeMap<String, String>,
    parser_configuration: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    project_search_roots: Option<Vec<String>>,
    #[serde(skip_serializing_if = "ProjectInventorySchema::is_default")]
    given_project_inventory: ProjectInventorySchema,
    files: Vec<String>,
    cache: Option<PathBuf>,
}
impl From<WorkerRequest> for RequestSchema {
    fn from(request: WorkerRequest) -> Self {
        Self {
            performance: request.performance.into(),
            rule_context: match &request.rule_context {
                RuleContextPreparation::Unconfigured => None,
                RuleContextPreparation::Prepared(context) => {
                    Some(serde_json::Value::from(RuleContextInput(context)))
                }
            },
            rule_observations: match &request.rule_observations {
                InputDeclaration::Undeclared => None,
                InputDeclaration::Declared(declarations) => {
                    Some(serde_json::Value::from(ObservationsInput(declarations)))
                }
            },
            project_context_mode: request.project_context_mode.into(),
            context_manifest: match request.context_manifest {
                ContextSeal::Unsealed => None,
                ContextSeal::Sealed(manifest) => Some(manifest.into()),
            },
            root: request.root,
            rules: request.rules,
            selected_rules: match request.selected_rules {
                CohortSelection::Frozen => None,
                CohortSelection::Explicit(rules) => {
                    Some(rules.into_iter().map(|id| id.to_string()).collect())
                }
            },
            query_modules: request
                .query_modules
                .into_iter()
                .map(|(name, path)| (name.0, path))
                .collect(),
            rule_applicability: request
                .rule_applicability
                .into_iter()
                .map(|(rule, policy)| (rule.0, policy.0))
                .collect(),
            parser_configuration: match request.parser_configuration {
                ParserSelection::Unspecified => None,
                ParserSelection::Named(name) => Some(name.0),
            },
            source_language: match request.source_language {
                ProviderSelection::Unspecified => None,
                ProviderSelection::Selected(name) => Some(name.0),
            },
            project_search_roots: match request.project_search_roots {
                SearchRoots::Undeclared => None,
                SearchRoots::Declared(roots) => {
                    Some(roots.into_iter().map(|root| root.0.into_string()).collect())
                }
            },
            given_project_inventory: request.given_project_inventory.into(),
            files: request
                .files
                .into_iter()
                .map(|path| path.0.into_string())
                .collect(),
            cache: match request.state {
                WorkerState::Fresh => None,
                WorkerState::Retained(path) => Some(path),
            },
        }
    }
}
/// Encode a typed candidate invocation for the child process.
impl TryFrom<WorkerRequest> for RequestJson {
    type Error = Error;
    fn try_from(request: WorkerRequest) -> Result<Self, Self::Error> {
        serde_json::to_string(&RequestSchema::from(request))
            .map(Self)
            .map_err(|error| Error::Json(Box::new(error)))
    }
}

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

/// Explicit project lookup mode on the worker wire.
#[derive(Clone, Copy, Debug, Default, Serialize)]
#[serde(rename_all = "kebab-case")]
enum ModeSchema {
    #[default]
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
        matches!(self.completeness, ProjectInventoryCompletenessSchema::Partial)
            && self.entries.is_empty()
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

/// Exact declared inventory used to validate context lookups.
#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
struct ManifestSchema {
    boundary: PathBuf,
    logical_root: PathBuf,
    auxiliary_files: BTreeMap<PathBuf, String>,
    source_files: BTreeMap<PathBuf, String>,
    missing: Vec<PathBuf>,
}
impl From<ContextManifest> for ManifestSchema {
    fn from(value: ContextManifest) -> Self {
        let digests = |files: BTreeMap<PathBuf, ArtifactDigest>| {
            files
                .into_iter()
                .map(|(path, digest)| (path, digest.0))
                .collect()
        };
        Self {
            boundary: value.boundary,
            logical_root: value.logical_root,
            auxiliary_files: digests(value.auxiliary_files),
            source_files: digests(value.source_files),
            missing: value.missing,
        }
    }
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
