//! JSON exists only at the child-process boundary.
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::BenchmarkFinding;
use ryo_benchmark_contracts::domain::reports::types::RunReport;
use ryo_benchmark_contracts::domain::requests::types::WorkerRequest;
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::context::types::ContextManifest;
use ryo_benchmark_contracts::domain::context::types::ProjectContextReceipt;
use ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};

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
    project_context_mode: ModeSchema,
    #[serde(default)]
    /// Sealed source, auxiliary and missing context inventory.
    context_manifest: Option<ManifestSchema>,
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
        skip_serializing_if = "ProjectInventorySchema::is_default"
    )]
    given_project_inventory:
        ProjectInventorySchema,
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
    project_context: Option<ReceiptSchema>,
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
            rule_context: match &request.rule_context {
                ryo_benchmark_contracts::domain::requests::types::RuleContextPreparation::Prepared(prepared) => {
                    Some(crate::outbound::worker::schemas::prepared_value(prepared))
                }
                ryo_benchmark_contracts::domain::requests::types::RuleContextPreparation::Unconfigured => None,
            },
            rule_observations: match &request.rule_observations {
                ryo_benchmark_contracts::domain::requests::types::InputDeclaration::Declared(declared) => {
                    Some(crate::outbound::worker::schemas::observation_declarations_value(declared))
                }
                ryo_benchmark_contracts::domain::requests::types::InputDeclaration::Undeclared => None,
            },
            project_context_mode: request.project_context_mode.into(),
            context_manifest: match request.context_manifest {
                ryo_benchmark_contracts::domain::requests::types::ContextSeal::Sealed(manifest) => Some(manifest.into()),
                ryo_benchmark_contracts::domain::requests::types::ContextSeal::Unsealed => None,
            },
            root: request.root,
            rules: request.rules,
            selected_rules: match request.selected_rules {
                ryo_benchmark_contracts::domain::requests::types::CohortSelection::Explicit(rules) => {
                    Some(rules.into_iter().map(|id| id.to_string()).collect())
                }
                ryo_benchmark_contracts::domain::requests::types::CohortSelection::Frozen => None,
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
                ryo_benchmark_contracts::domain::requests::types::ParserSelection::Named(name) => Some(name.0),
                ryo_benchmark_contracts::domain::requests::types::ParserSelection::Unspecified => None,
            },
            source_language: match request.source_language {
                ryo_benchmark_contracts::domain::requests::types::ProviderSelection::Selected(name) => Some(name.0),
                ryo_benchmark_contracts::domain::requests::types::ProviderSelection::Unspecified => None,
            },
            project_search_roots: match request.project_search_roots {
                ryo_benchmark_contracts::domain::requests::types::SearchRoots::Declared(roots) => {
                    Some(roots.into_iter().map(|root| root.0.into_string()).collect())
                }
                ryo_benchmark_contracts::domain::requests::types::SearchRoots::Undeclared => None,
            },
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
            project_context: wire
                .project_context
                .map_or(ryo_benchmark_contracts::domain::reports::types::ProjectContext::NotRequested, |receipt| ryo_benchmark_contracts::domain::reports::types::ProjectContext::Observed(receipt.into())),
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
            coverage: wire.coverage.map_or(ryo_benchmark_contracts::domain::reports::types::CoverageAccounting::Unaccounted, |coverage| {
                ryo_benchmark_contracts::domain::reports::types::CoverageAccounting::Accounted(ryo_benchmark_contracts::domain::reports::types::RuleCoverage {
                    selected: ryo_benchmark_contracts::domain::reports::types::PairCount(coverage.selected),
                    evaluated: ryo_benchmark_contracts::domain::reports::types::PairCount(coverage.evaluated),
                    skipped: ryo_benchmark_contracts::domain::reports::types::PairCount(coverage.skipped),
                    unknown_or_unread: ryo_benchmark_contracts::domain::reports::types::PairCount(coverage.unknown_or_unread),
                })
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
        ryo_benchmark_contracts::domain::reports::service::require_complete_execution(&report)?;
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
/// Explicit project lookup mode of a worker request and report.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
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
struct ProjectEntrySchema {
    path: String,
    state: ProjectEntryStateSchema,
}

/// The declared project inventory of a request.
#[derive(Clone, Debug, Default, Serialize)]
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

/// The sealed context inventory a worker request declares.
#[derive(Clone, Debug, Serialize)]
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

/// The actual dependency observations a worker report carries.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ReceiptSchema {
    mode: ModeSchema,
    logical_root: PathBuf,
    actual_root: PathBuf,
    manifest_hash: String,
    dependencies: BTreeMap<PathBuf, Option<[u8; 16]>>,
}
impl From<ReceiptSchema> for ProjectContextReceipt {
    fn from(wire: ReceiptSchema) -> Self {
        Self {
            mode: wire.mode.into(),
            logical_root: wire.logical_root,
            actual_root: wire.actual_root,
            manifest_hash: ArtifactDigest(wire.manifest_hash),
            dependencies: wire
                .dependencies
                .into_iter()
                .map(|(path, state)| (path, state.map(ryo::linter::DependencyFingerprint)))
                .collect(),
        }
    }
}
#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
