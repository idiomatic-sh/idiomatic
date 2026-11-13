//! Cargo's documents: the workspace metadata, the build's compiler-artifact messages, and the seal record written beside a sealed executable.
use super::constants::{LOCKED, ONE_BENCH_EXECUTABLE};
use cargo_metadata::{Message, MetadataCommand, TargetKind};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use serde::Serialize;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

/// The build target directory of the workspace in the current directory.
pub(super) fn target_directory(cargo: &OsStr) -> Result<PathBuf, Error> {
    MetadataCommand::new()
        .cargo_path(cargo)
        .no_deps()
        .other_options(vec![LOCKED.to_owned()])
        .exec()
        .map(|metadata| metadata.target_directory.into_std_path_buf())
        .map_err(|error| Error::Invalid(error.to_string()))
}

/// The one bench executable of the package at `manifest` among a build's messages: a compiler artifact of
/// that package whose target is a bench built as a test harness.
pub(super) fn bench_executable(messages: &[u8], manifest: &Path) -> Result<PathBuf, Error> {
    let mut found = Vec::new();
    for message in Message::parse_stream(messages) {
        if let Message::CompilerArtifact(artifact) = message?
            && artifact.manifest_path.as_std_path() == manifest
            && artifact.target.is_kind(TargetKind::Bench)
            && artifact.profile.test
            && let Some(executable) = artifact.executable
        {
            found.push(executable.into_std_path_buf());
        }
    }
    match <[PathBuf; 1]>::try_from(found) {
        Ok([executable]) => Ok(executable),
        Err(_) => Err(Error::Invalid(ONE_BENCH_EXECUTABLE.into())),
    }
}

/// The record of one sealed executable: where its bytes came from and their digest.
#[derive(Serialize)]
struct SealSchema<'a> {
    sha256: &'a str,
    source_artifact: &'a Path,
}

/// The seal record of an executable copied from `source` with this digest, one JSON line.
pub(super) fn seal_record(source: &Path, sha256: &str) -> Result<Vec<u8>, Error> {
    let mut record = serde_json::to_vec(&SealSchema {
        sha256,
        source_artifact: source,
    })
    .map_err(|error| Error::Json(Box::new(error)))?;
    record.push(b'\n');
    Ok(record)
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
