//! Pure identity of declared file observations, separate from unchanged source-byte framing.
use super::constants;
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
                    constants::METADATA_NAMES_AN_UNDECLARED_SOURCE.into(),
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
            constants::REQUIRED_FILE_OBSERVATIONS_ARE_MISSING.into(),
        ));
    }
    Ok(())
}
