//! Explicit context wire values shared by the two worker schemas.
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::context::types::ContextManifest;
use ryo_benchmark_contracts::domain::context::types::ProjectContextReceipt;
use ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
/// Explicit project lookup mode on the worker wire.
pub(crate) enum ModeSchema {
    #[default]
    /// Source-only execution with no external project lookup.
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

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ProjectInventoryCompletenessSchema {
    #[default]
    Partial,
    CompleteWithinDeclaredScope,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ProjectEntryStateSchema {
    File,
    Directory,
    Other,
    Missing,
    Unknown,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProjectEntrySchema {
    path: String,
    state: ProjectEntryStateSchema,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProjectInventorySchema {
    #[serde(default)]
    completeness: ProjectInventoryCompletenessSchema,
    #[serde(default)]
    entries: Vec<ProjectEntrySchema>,
}

impl ProjectInventorySchema {
    pub(crate) fn is_default(&self) -> bool {
        matches!(
            self.completeness,
            ProjectInventoryCompletenessSchema::Partial
        ) && self.entries.is_empty()
    }
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

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
/// Exact declared inventory used to validate context lookups.
pub(crate) struct ManifestSchema {
    /// Physical boundary containing every declared context path.
    boundary: PathBuf,
    /// Stable run-root path within the context boundary.
    logical_root: PathBuf,
    /// Auxiliary relative paths and SHA-256 content seals.
    auxiliary_files: BTreeMap<PathBuf, String>,
    /// Input source paths and SHA-256 content seals.
    source_files: BTreeMap<PathBuf, String>,
    /// Explicitly declared absent context paths.
    missing: Vec<PathBuf>,
}
impl From<ManifestSchema> for ContextManifest {
    fn from(wire: ManifestSchema) -> Self {
        Self {
            boundary: wire.boundary,
            logical_root: wire.logical_root,
            auxiliary_files: wire
                .auxiliary_files
                .into_iter()
                .map(|(path, digest)| (path, ArtifactDigest(digest)))
                .collect(),
            source_files: wire
                .source_files
                .into_iter()
                .map(|(path, digest)| (path, ArtifactDigest(digest)))
                .collect(),
            missing: wire.missing,
        }
    }
}
impl From<ContextManifest> for ManifestSchema {
    fn from(value: ContextManifest) -> Self {
        Self {
            boundary: value.boundary,
            logical_root: value.logical_root,
            auxiliary_files: value
                .auxiliary_files
                .into_iter()
                .map(|(path, digest)| (path, digest.0))
                .collect(),
            source_files: value
                .source_files
                .into_iter()
                .map(|(path, digest)| (path, digest.0))
                .collect(),
            missing: value.missing,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
/// Actual dependency observations and the manifest identity that governed them.
pub(crate) struct ReceiptSchema {
    /// Project context mode used for this candidate.
    mode: ModeSchema,
    /// Stable run-root path within the context boundary.
    logical_root: PathBuf,
    /// Physical run root of this invocation.
    actual_root: PathBuf,
    /// Stable content seal of the declared context inventory.
    manifest_hash: String,
    /// Actual dependency paths and optional opaque content fingerprints.
    dependencies: BTreeMap<PathBuf, Option<[u8; 16]>>,
}
impl From<ProjectContextReceipt> for ReceiptSchema {
    fn from(value: ProjectContextReceipt) -> Self {
        Self {
            mode: value.mode.into(),
            logical_root: value.logical_root,
            actual_root: value.actual_root,
            manifest_hash: value.manifest_hash.0,
            dependencies: value
                .dependencies
                .into_iter()
                .map(|(path, state)| (path, state.map(|fingerprint| fingerprint.0)))
                .collect(),
        }
    }
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
