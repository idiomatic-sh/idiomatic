//! The worker request document: the JSON a parent process passes on the command line.
use crate::outbound::worker::adapters::json::types::{DocumentWire, WorkerPayload};
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode;
use ryo_benchmark_contracts::domain::context::types::rule_observation_declarations::value::RuleObservationDeclarations;
use ryo_benchmark_contracts::domain::context::types::{ContextManifest, PreparedRuleContext};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::{
    ApplicabilityPolicy, CohortSelection, ContextSeal, DiagnosticOptions, DiagnosticSwitch,
    InputDeclaration, ParserConfigurationName, ParserSelection, PerformanceOptions,
    ProviderSelection, QueryModuleName, RuleContextPreparation, RuleLabel, SearchRoots,
    SourceLanguageName, WorkerRequest, WorkerState,
};
use serde::Deserialize;
use std::{collections::BTreeMap, path::PathBuf};

/// Explicit candidate invocation at the JSON process boundary.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RequestSchema {
    #[serde(default)]
    performance: OptionsSchema,
    #[serde(default)]
    rule_context: Option<serde_json::Value>,
    #[serde(default)]
    rule_observations: Option<serde_json::Value>,
    #[serde(default)]
    selected_rules: Option<Vec<String>>,
    #[serde(default)]
    project_context_mode: ModeSchema,
    #[serde(default)]
    context_manifest: Option<ManifestSchema>,
    root: PathBuf,
    rules: PathBuf,
    #[serde(default)]
    query_modules: BTreeMap<String, PathBuf>,
    #[serde(default)]
    rule_applicability: BTreeMap<String, String>,
    #[serde(default)]
    parser_configuration: Option<String>,
    #[serde(default)]
    source_language: Option<String>,
    #[serde(default)]
    project_search_roots: Option<Vec<String>>,
    #[serde(default)]
    given_project_inventory: ProjectInventorySchema,
    files: Vec<String>,
    cache: Option<PathBuf>,
}

impl TryFrom<WorkerPayload<'_>> for WorkerRequest {
    type Error = Error;
    fn try_from(payload: WorkerPayload<'_>) -> Result<Self, Self::Error> {
        let text = payload
            .0
            .to_str()
            .ok_or_else(|| Error::Invalid("worker request is not UTF-8".into()))?;
        let request: RequestSchema =
            serde_json::from_str(text).map_err(|error| Error::Json(Box::new(error)))?;
        request.try_into()
    }
}

impl TryFrom<RequestSchema> for WorkerRequest {
    type Error = Error;
    fn try_from(request: RequestSchema) -> Result<Self, Self::Error> {
        Ok(Self {
            performance: request.performance.into(),
            selected_rules: match request.selected_rules {
                None => CohortSelection::Frozen,
                Some(rules) => CohortSelection::Explicit(
                    rules
                        .into_iter()
                        .map(|id| {
                            ryo::linter::RuleId::try_from(id.as_str())
                                .map_err(|error| Error::Invalid(format!("invalid selected rule: {error}")))
                        })
                        .collect::<Result<_, Error>>()?,
                ),
            },
            rule_context: match request.rule_context {
                None => RuleContextPreparation::Unconfigured,
                Some(value) => RuleContextPreparation::Prepared(PreparedRuleContext::try_from(
                    DocumentWire(value),
                )?),
            },
            rule_observations: match request.rule_observations {
                None => InputDeclaration::Undeclared,
                Some(value) => InputDeclaration::Declared(RuleObservationDeclarations::try_from(
                    DocumentWire(value),
                )?),
            },
            project_search_roots: match request.project_search_roots {
                None => SearchRoots::Undeclared,
                Some(roots) => SearchRoots::Declared(
                    roots
                        .into_iter()
                        .map(|root| ryo::linter::RelativePath(root.into_boxed_str()))
                        .collect(),
                ),
            },
            given_project_inventory: request.given_project_inventory.into(),
            project_context_mode: request.project_context_mode.into(),
            context_manifest: request
                .context_manifest
                .map_or(ContextSeal::Unsealed, |manifest| {
                    ContextSeal::Sealed(manifest.into())
                }),
            root: request.root,
            rules: request.rules,
            query_modules: request
                .query_modules
                .into_iter()
                .map(|(name, path)| (QueryModuleName(name), path))
                .collect(),
            rule_applicability: request
                .rule_applicability
                .into_iter()
                .map(|(rule, policy)| (RuleLabel(rule), ApplicabilityPolicy(policy)))
                .collect(),
            parser_configuration: request.parser_configuration.map_or(
                ParserSelection::Unspecified,
                |name| ParserSelection::Named(ryo_benchmark_contracts::domain::requests::types::ParserConfigurationName(name)),
            ),
            source_language: request.source_language.map_or(
                ProviderSelection::Unspecified,
                |name| ProviderSelection::Selected(ryo_benchmark_contracts::domain::requests::types::SourceLanguageName(name)),
            ),
            files: request
                .files
                .into_iter()
                .map(|path| ryo::linter::RelativePath(path.into_boxed_str()))
                .collect(),
            state: request.cache.map_or(WorkerState::Fresh, WorkerState::Retained),
        })
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct OptionsSchema {
    diagnostics: DiagnosticsSchema,
}

#[derive(Clone, Copy, Debug, Default, Deserialize)]
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

impl From<OptionsSchema> for PerformanceOptions {
    fn from(value: OptionsSchema) -> Self {
        let switch = |on: bool| if on { DiagnosticSwitch::On } else { DiagnosticSwitch::Off };
        let diagnostics = value.diagnostics;
        Self {
            diagnostics: DiagnosticOptions {
                execution: switch(diagnostics.execution),
                lifecycle_memory: switch(diagnostics.lifecycle_memory),
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

/// Explicit project lookup mode of a worker request.
#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum ModeSchema {
    /// Source-only execution with no external project lookup.
    #[default]
    Isolated,
    /// Disk lookup constrained by the sealed context manifest.
    ListedDisk,
}
impl From<ModeSchema> for ProjectContextMode {
    fn from(mode: ModeSchema) -> Self {
        match mode {
            ModeSchema::Isolated => Self::Isolated,
            ModeSchema::ListedDisk => Self::ListedDisk,
        }
    }
}

/// How complete the declared project inventory is.
#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum ProjectInventoryCompletenessSchema {
    #[default]
    Partial,
    CompleteWithinDeclaredScope,
}

/// What a declared project entry is.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum ProjectEntryStateSchema {
    File,
    Directory,
    Other,
    Missing,
    Unknown,
}

/// One declared project entry.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectEntrySchema {
    path: String,
    state: ProjectEntryStateSchema,
}

/// The declared project inventory of a request.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectInventorySchema {
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

/// The sealed context inventory a worker request declares.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestSchema {
    boundary: PathBuf,
    logical_root: PathBuf,
    auxiliary_files: BTreeMap<PathBuf, String>,
    source_files: BTreeMap<PathBuf, String>,
    missing: Vec<PathBuf>,
}
impl From<ManifestSchema> for ContextManifest {
    fn from(wire: ManifestSchema) -> Self {
        let digests = |files: BTreeMap<PathBuf, String>| {
            files
                .into_iter()
                .map(|(path, digest)| (path, ArtifactDigest(digest)))
                .collect()
        };
        Self {
            boundary: wire.boundary,
            logical_root: wire.logical_root,
            auxiliary_files: digests(wire.auxiliary_files),
            source_files: digests(wire.source_files),
            missing: wire.missing,
        }
    }
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
