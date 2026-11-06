//! The registered input identities a case declares: file metadata and the project namespace scope.
use super::errors::RegisteredInputsError;
use super::types::{ObservationDeclaration, RefusalText, RegisteredInputs};
use ryo_benchmark_contracts::domain::artifacts::types::{
    FileMetadataEntry, digest::value::ArtifactDigest,
    relative_artifact_path::value::RelativeArtifactPath,
};
use std::path::Path;

fn refused(error: impl std::fmt::Display) -> RegisteredInputsError {
    RegisteredInputsError::Refused(RefusalText(error.to_string()))
}

/// Check the declared metadata against the declared files and recompute its pure framed identity.
pub(crate) fn metadata_digest(
    inputs: &RegisteredInputs,
) -> Result<Option<ArtifactDigest>, RegisteredInputsError> {
    let required = inputs.observation == ObservationDeclaration::Required;
    if let Some(metadata) = &inputs.metadata {
        for path in metadata.keys() {
            RelativeArtifactPath::try_from(Path::new(path.0.as_ref())).map_err(refused)?;
            if !inputs.files.contains(path) {
                return Err(RegisteredInputsError::UndeclaredMetadataSource);
            }
        }
    }
    if required
        && inputs
            .metadata
            .as_ref()
            .is_none_or(|values| values.len() != inputs.files.len())
    {
        return Err(RegisteredInputsError::InputObservationsMissing);
    }
    ArtifactDigest::file_metadata(inputs.metadata.as_ref().map(|values| {
        values.iter().map(|(path, executable)| FileMetadataEntry {
            path,
            executable: *executable,
        })
    }))
    .map_err(refused)
}

/// Recompute the project input identity from the registered explicit roots and entry inventory.
pub(crate) fn project_inputs_digest(
    inputs: &RegisteredInputs,
) -> Result<Option<ArtifactDigest>, RegisteredInputsError> {
    for root in inputs.search_roots.iter().flatten() {
        if !root.0.is_empty() {
            RelativeArtifactPath::try_from(Path::new(root.0.as_ref())).map_err(refused)?;
        }
    }
    for entry in &inputs.inventory.entries {
        RelativeArtifactPath::try_from(Path::new(entry.path.0.as_ref())).map_err(refused)?;
    }
    crate::domain::suite::project_inputs::digest(inputs.search_roots.as_deref(), &inputs.inventory)
        .map_err(refused)
}
