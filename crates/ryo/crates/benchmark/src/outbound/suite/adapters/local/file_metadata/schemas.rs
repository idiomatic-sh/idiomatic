//! The shape of the execute bit the filesystem reports for a source file, and its identity digest.
use crate::outbound::suite::adapters::local::constants;
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::artifacts::types::relative_artifact_path::value::RelativeArtifactPath;
use ryo_benchmark_contracts::domain::artifacts::types::{FileExecutable, FileMetadataEntry};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::RunRequest;
use std::path::Path;

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;

#[cfg(unix)]
pub(in super::super) fn observed(path: &Path) -> Result<bool, Error> {
    use std::os::unix::fs::PermissionsExt as _;
    let metadata = std::fs::metadata(path)?;
    if !metadata.is_file() {
        return Err(Error::Invalid(
            constants::METADATA_SOURCE_IS_NOT_A_REGULAR.into(),
        ));
    }
    Ok(metadata.permissions().mode() & 0o111 != 0)
}
#[cfg(not(unix))]
pub(in super::super) fn observed(_: &Path) -> Result<bool, Error> {
    Err(Error::Invalid(
        constants::POSIX_EXECUTABLE_OBSERVATIONS_ARE_UNSUPPORTE.into(),
    ))
}

/// Recompute a complete observation identity from the current physical sources.
pub(in super::super) fn actual_digest(root: &Path, files: &[std::path::PathBuf]) -> Result<String, Error> {
    let root = root.canonicalize()?;
    let mut values = std::collections::BTreeMap::new();
    for file in files {
        let relative = RelativeArtifactPath::try_from(file.as_path())?;
        let path = root.join(relative.as_ref()).canonicalize()?;
        if !path.starts_with(&root) {
            return Err(Error::Invalid(
                constants::METADATA_SOURCE_ESCAPES_INPUT_ROOT.into(),
            ));
        }
        let name = file
            .to_str()
            .ok_or_else(|| Error::Invalid(constants::METADATA_SOURCE_NAME_IS_NOT_UTF8.into()))?;
        if values
            .insert(ryo::linter::RelativePath(name.into()), observed(&path)?)
            .is_some()
        {
            return Err(Error::Invalid(
                constants::DUPLICATE_PHYSICAL_METADATA_SOURCE.into(),
            ));
        }
    }
    ArtifactDigest::file_metadata((Some(&values)).map(|values| {
        values.iter().map(|(path, executable)| FileMetadataEntry {
            path,
            executable: if *executable {
                FileExecutable::Set
            } else {
                FileExecutable::Unset
            },
        })
    }))?
    .map(|digest| digest.0)
    .ok_or_else(|| Error::Invalid(constants::MISSING_PHYSICAL_METADATA_DIGEST.into()))
}

/// Stage only the declared execute-bit observation; preserve other permission bits.
pub(in super::super) fn stage_executable(path: &Path, executable: bool) -> Result<(), Error> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let metadata = std::fs::metadata(path)?;
        let mode = metadata.permissions().mode();
        let mode = if executable {
            mode | 0o100
        } else {
            mode & !0o111
        };
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))?;
    }
    if observed(path)? != executable {
        return Err(Error::Invalid(
            constants::STAGED_EXECUTABLE_METADATA_DIFFERS.into(),
        ));
    }
    Ok(())
}

/// Unconditional per-attempt verification, including Isolated input mode.
pub(in super::super) fn validate(request: &RunRequest) -> Result<(), Error> {
    let ryo_benchmark_contracts::domain::requests::types::FileMetadataDeclaration::Declared(values) = &request.file_metadata else {
        return Ok(());
    };
    let root = request.root.canonicalize()?;
    for (file, expected) in values {
        let relative = RelativeArtifactPath::try_from(Path::new(file.0.as_ref()))?;
        if !request.files.contains(file) {
            return Err(Error::Invalid(
                constants::METADATA_NAMES_AN_UNDECLARED_SOURCE.into(),
            ));
        }
        let path = root.join(relative.as_ref()).canonicalize()?;
        if !path.starts_with(&root) {
            return Err(Error::Invalid(
                constants::METADATA_SOURCE_ESCAPES_INPUT_ROOT.into(),
            ));
        }
        let actual = if observed(&path)? {
            FileExecutable::Set
        } else {
            FileExecutable::Unset
        };
        if actual != *expected {
            return Err(Error::Invalid(
                constants::DECLARED_EXECUTABLE_METADATA_DRIFTED_BEFORE.into(),
            ));
        }
    }
    Ok(())
}
