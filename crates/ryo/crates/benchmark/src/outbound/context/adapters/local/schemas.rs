//! Explicit context wire values shared by the two worker schemas.
use crate::domain::{
    diagnostics::types::ProjectContextReceipt,
    suite::types::{ArtifactDigest, ContextManifest, ProjectContextMode},
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ModeSchema {
    #[default]
    Isolated,
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

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ManifestSchema {
    boundary: PathBuf,
    logical_root: PathBuf,
    auxiliary_files: BTreeMap<PathBuf, String>,
    source_files: BTreeMap<PathBuf, String>,
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
pub(crate) struct ReceiptSchema {
    mode: ModeSchema,
    logical_root: PathBuf,
    actual_root: PathBuf,
    manifest_hash: String,
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
