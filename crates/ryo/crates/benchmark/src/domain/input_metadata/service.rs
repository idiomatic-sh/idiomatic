//! Pure identity of declared file observations, separate from unchanged source-byte framing.
use super::errors::InputMetadataError;
use ryo::linter::RelativePath;
use ryo_benchmark_contracts::domain::artifacts::types::relative_artifact_path::value::RelativeArtifactPath;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::FileMetadataDeclaration;
use ryo_benchmark_contracts::domain::workloads::types::FileObservation;

/// A required observation must be explicit for every declared source, even for negative cases.
///
/// # Errors
/// Refuses metadata naming an unsafe or undeclared source, and a required observation that is
/// missing or does not cover every declared source.
pub fn require(
    requirement: FileObservation,
    files: &[RelativePath],
    metadata: &FileMetadataDeclaration,
) -> Result<(), Error> {
    if let FileMetadataDeclaration::Declared(values) = metadata {
        for path in values.keys() {
            RelativeArtifactPath::try_from(std::path::Path::new(path.0.as_ref()))?;
            if !files.contains(path) {
                return Err(Error::Invalid(
                    InputMetadataError::UndeclaredSource.to_string(),
                ));
            }
        }
    }
    if requirement == FileObservation::PosixExecutable
        && match metadata {
            FileMetadataDeclaration::Undeclared => true,
            FileMetadataDeclaration::Declared(values) => {
                values.len() != files.len() || files.iter().any(|file| !values.contains_key(file))
            }
        }
    {
        return Err(Error::Invalid(
            InputMetadataError::RequiredObservationsMissing.to_string(),
        ));
    }
    Ok(())
}

/// The identity of the declared file metadata: absent when the case declares none.
///
/// # Errors
/// Refuses duplicate or unordered paths and paths the canonical framing cannot hold.
pub fn digest(
    metadata: &FileMetadataDeclaration,
) -> Result<
    Option<ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest>,
    ryo_benchmark_contracts::domain::artifacts::errors::ArtifactsError,
> {
    ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest::file_metadata(
        match metadata {
            FileMetadataDeclaration::Undeclared => None,
            FileMetadataDeclaration::Declared(values) => {
                Some(values.iter().map(|(path, executable)| {
                    ryo_benchmark_contracts::domain::artifacts::types::FileMetadataEntry {
                        path,
                        executable: *executable,
                    }
                }))
            }
        },
    )
}
