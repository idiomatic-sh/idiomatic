//! Physical observation and staging, confined to the suite's filesystem edge.
use ryo_benchmark_contracts::domain::artifacts::types::{FileExecutable, FileMetadataEntry};
use ryo_benchmark_contracts::domain::artifacts::types::{
    digest::value::ArtifactDigest, relative_artifact_path::value::RelativeArtifactPath,
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::RunRequest;
use std::path::Path;

#[cfg(unix)]
fn observed(path: &Path) -> Result<bool, Error> {
    use std::os::unix::fs::PermissionsExt as _;
    let metadata = std::fs::metadata(path)?;
    if !metadata.is_file() {
        return Err(Error::Invalid(
            crate::outbound::suite::adapters::local::constants::METADATA_SOURCE_IS_NOT_A_REGULAR.into(),
        ));
    }
    Ok(metadata.permissions().mode() & 0o111 != 0)
}
#[cfg(not(unix))]
fn observed(_: &Path) -> Result<bool, Error> {
    Err(Error::Invalid(
        crate::outbound::suite::adapters::local::constants::POSIX_EXECUTABLE_OBSERVATIONS_ARE_UNSUPPORTE.into(),
    ))
}

/// Stage only the declared execute-bit observation; preserve other permission bits.
pub(super) fn stage(path: &Path, executable: bool) -> Result<(), Error> {
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
        return Err(Error::Invalid(crate::outbound::suite::adapters::local::constants::STAGED_EXECUTABLE_METADATA_DIFFERS.into()));
    }
    Ok(())
}

/// Unconditional per-attempt verification, including Isolated input mode.
pub(super) fn validate(request: &RunRequest) -> Result<(), Error> {
    let Some(values) = &request.file_metadata else {
        return Ok(());
    };
    let root = request.root.canonicalize()?;
    for (file, expected) in values {
        let relative = RelativeArtifactPath::try_from(Path::new(file.0.as_ref()))?;
        if !request.files.contains(file) {
            return Err(Error::Invalid(crate::outbound::suite::adapters::local::constants::METADATA_NAMES_AN_UNDECLARED_SOURCE.into()));
        }
        let path = root.join(relative.as_ref()).canonicalize()?;
        if !path.starts_with(&root) {
            return Err(Error::Invalid(crate::outbound::suite::adapters::local::constants::METADATA_SOURCE_ESCAPES_INPUT_ROOT.into()));
        }
        if observed(&path)? != *expected {
            return Err(Error::Invalid(
                crate::outbound::suite::adapters::local::constants::DECLARED_EXECUTABLE_METADATA_DRIFTED_BEFORE.into(),
            ));
        }
    }
    Ok(())
}

/// Recompute a complete observation identity from the current physical sources.
pub(super) fn actual_digest(root: &Path, files: &[std::path::PathBuf]) -> Result<String, Error> {
    let root = root.canonicalize()?;
    let mut values = std::collections::BTreeMap::new();
    for file in files {
        let relative = RelativeArtifactPath::try_from(file.as_path())?;
        let path = root.join(relative.as_ref()).canonicalize()?;
        if !path.starts_with(&root) {
            return Err(Error::Invalid(crate::outbound::suite::adapters::local::constants::METADATA_SOURCE_ESCAPES_INPUT_ROOT.into()));
        }
        let name = file
            .to_str()
            .ok_or_else(|| Error::Invalid(crate::outbound::suite::adapters::local::constants::METADATA_SOURCE_NAME_IS_NOT_UTF8.into()))?;
        if values
            .insert(ryo::linter::RelativePath(name.into()), observed(&path)?)
            .is_some()
        {
            return Err(Error::Invalid(crate::outbound::suite::adapters::local::constants::DUPLICATE_PHYSICAL_METADATA_SOURCE.into()));
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
    .ok_or_else(|| Error::Invalid(crate::outbound::suite::adapters::local::constants::MISSING_PHYSICAL_METADATA_DIGEST.into()))
}

/// Write only selected source facts; semantic preparation and expected reports stay above this port.
pub(super) fn materialize(
    boundary: &Path,
    sources: &ryo_benchmark_contracts::domain::workloads::types::Case,
) -> Result<crate::outbound::suite::types::MaterializedCase, Error> {
    let context_manifest = sources
        .context
        .as_ref()
        .map(|context| {
            crate::outbound::context::adapters::local::adapter::materialize(
                boundary,
                context,
                &sources.files,
            )
        })
        .transpose()?;
    let root = context_manifest.as_ref().map_or_else(
        || boundary.to_owned(),
        |manifest| manifest.boundary.join(&manifest.logical_root),
    );
    std::fs::create_dir_all(&root)?;
    for (file, source) in &sources.files {
        let relative = RelativeArtifactPath::try_from(Path::new(file.0.as_ref()))?;
        let path = root.join(relative.as_ref());
        std::fs::create_dir_all(
            path.parent()
                .ok_or_else(|| Error::Invalid(super::constants::MISSING_CASE_PARENT.into()))?,
        )?;
        std::fs::write(&path, source.0.as_bytes())?;
        if let Some(executable) = sources
            .file_metadata
            .as_ref()
            .and_then(|values| values.get(file))
        {
            stage(&path, *executable)?;
        }
    }
    Ok(crate::outbound::suite::types::MaterializedCase {
        root,
        context_manifest,
    })
}

/// Materialize only the declared Given entries and reject contradictions with written sources.
pub(super) fn materialize_inventory(
    root: &Path,
    inventory: &ryo::linter::GivenProjectInventory,
) -> Result<(), Error> {
    for entry in &inventory.entries {
        let relative = RelativeArtifactPath::try_from(Path::new(entry.path.0.as_ref()))?;
        let path = root.join(relative.as_ref());
        match entry.state {
            ryo::linter::ProjectEntryState::File if !path.is_file() => {
                return Err(Error::Invalid(
                    crate::outbound::suite::adapters::local::constants::GIVEN_PROJECT_FILE_ENTRY_LACKS_STAGED.into(),
                ));
            }
            ryo::linter::ProjectEntryState::Directory => {
                if path.is_file() {
                    return Err(Error::Invalid(
                        crate::outbound::suite::adapters::local::constants::GIVEN_PROJECT_DIRECTORY_COLLIDES_WITH_A.into(),
                    ));
                }
                std::fs::create_dir_all(&path)?;
            }
            ryo::linter::ProjectEntryState::Missing if path.exists() => {
                return Err(Error::Invalid(
                    crate::outbound::suite::adapters::local::constants::GIVEN_PROJECT_MISSING_ENTRY_COLLIDES_WITH.into(),
                ));
            }
            ryo::linter::ProjectEntryState::Other => {
                return Err(Error::Invalid(
                    crate::outbound::suite::adapters::local::constants::GIVEN_PROJECT_NON_FILE_ENTRY_REQUIRES.into(),
                ));
            }
            ryo::linter::ProjectEntryState::File
            | ryo::linter::ProjectEntryState::Missing
            | ryo::linter::ProjectEntryState::Unknown => {}
        }
    }
    Ok(())
}
