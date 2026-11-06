//! The registered input identities a case declares: file metadata and the project namespace scope.
use super::constants;
use super::errors::RegisteredInputsError;
use super::types::RefusalText;
use ryo_benchmark_contracts::domain::artifacts::types::{FileExecutable, FileMetadataEntry};
use serde_json::Value;

/// Decode only declared metadata and recompute its pure framed identity.
pub(crate) fn metadata_digest(case: &Value) -> Result<Value, RegisteredInputsError> {
    let files = case[constants::FILES]
        .as_array()
        .ok_or(RegisteredInputsError::RegisteredFilesMissing)?;
    let required = !case[constants::FILE_OBSERVATION].is_null();
    if required && case[constants::FILE_OBSERVATION] != constants::POSIX_EXECUTABLE {
        return Err(RegisteredInputsError::UnsupportedFileObservation);
    }
    let metadata = if case[constants::FILE_METADATA].is_null() {
        None
    } else {
        let values = case[constants::FILE_METADATA]
            .as_object()
            .ok_or(RegisteredInputsError::FileMetadataNotObject)?;
        let mut parsed = std::collections::BTreeMap::new();
        for (path, value) in values {
            ryo_benchmark_contracts::domain::artifacts::types::relative_artifact_path::value::RelativeArtifactPath::try_from(std::path::Path::new(std::path::Path::new(path)))
                .map_err(refused)?;
            if !files.iter().any(|file| file.as_str() == Some(path)) {
                return Err(RegisteredInputsError::UndeclaredMetadataSource);
            }
            parsed.insert(
                ryo::linter::RelativePath(path.clone().into_boxed_str()),
                value
                    .as_bool()
                    .ok_or(RegisteredInputsError::ExecutableNotBoolean)?,
            );
        }
        Some(parsed)
    };
    if required
        && metadata
            .as_ref()
            .is_none_or(|values| values.len() != files.len())
    {
        return Err(RegisteredInputsError::InputObservationsMissing);
    }
    Ok(serde_json::json!(
        ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest::file_metadata((metadata.as_ref()).map(|values| values.iter().map(|(path, executable)| FileMetadataEntry { path, executable: if *executable { FileExecutable::Set } else { FileExecutable::Unset } })))
            .map_err(refused)?
            .map(|value| value.0)
    ))
}

/// Recompute the project input identity from the registered explicit roots and entry inventory.
pub(crate) fn project_inputs_digest(case: &Value) -> Result<Value, RegisteredInputsError> {
    let roots = if case[constants::PROJECT_SEARCH_ROOTS].is_null() {
        None
    } else {
        Some(
            case[constants::PROJECT_SEARCH_ROOTS]
                .as_array()
                .ok_or(RegisteredInputsError::SearchRootsNotArray)?
                .iter()
                .map(|value| {
                    let root = value.as_str().ok_or(RegisteredInputsError::SearchRootNotText)?;
                    if !root.is_empty() {
                        ryo_benchmark_contracts::domain::artifacts::types::relative_artifact_path::value::RelativeArtifactPath::try_from(std::path::Path::new(root))
                            .map_err(refused)?;
                    }
                    Ok(ryo::linter::RelativePath(root.to_owned().into_boxed_str()))
                })
                .collect::<Result<Vec<_>, RegisteredInputsError>>()?,
        )
    };
    let inventory = if case[constants::GIVEN_PROJECT_INVENTORY].is_null() {
        ryo::linter::GivenProjectInventory::default()
    } else {
        let wire: crate::outbound::context::adapters::local::schemas::ProjectInventorySchema =
            serde_json::from_value(case[constants::GIVEN_PROJECT_INVENTORY].clone())
                .map_err(refused)?;
        wire.into()
    };
    for entry in &inventory.entries {
        ryo_benchmark_contracts::domain::artifacts::types::relative_artifact_path::value::RelativeArtifactPath::try_from(std::path::Path::new(entry.path.0.as_ref()))
            .map_err(refused)?;
    }
    Ok(serde_json::json!(
        crate::domain::suite::project_inputs::digest(roots.as_deref(), &inventory)
            .map_err(refused)?
            .map(|digest| digest.0)
    ))
}

fn refused(error: impl std::fmt::Display) -> RegisteredInputsError {
    RegisteredInputsError::Refused(RefusalText(error.to_string()))
}
