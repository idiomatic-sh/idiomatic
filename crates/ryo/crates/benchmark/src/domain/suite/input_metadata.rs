//! Pure identity of declared file observations, separate from unchanged source-byte framing.
use ryo::linter::RelativePath;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use std::collections::BTreeMap;

/// A required observation must be explicit for every declared source, even for negative cases.
pub fn require(
    requirement: Option<ryo_benchmark_contracts::domain::workloads::types::FileObservation>,
    files: &[RelativePath],
    metadata: Option<&BTreeMap<RelativePath, bool>>,
) -> Result<(), Error> {
    if let Some(values) = metadata {
        for path in values.keys() {
            ryo_benchmark_contracts::domain::artifacts::types::relative_artifact_path::value::RelativeArtifactPath::try_from(std::path::Path::new(path.0.as_ref()))?;
            if !files.contains(path) {
                return Err(Error::Invalid(crate::domain::suite::constants::METADATA_NAMES_AN_UNDECLARED_SOURCE.into()));
            }
        }
    }
    if requirement.is_some()
        && metadata.is_none_or(|values| {
            values.len() != files.len() || files.iter().any(|file| !values.contains_key(file))
        })
    {
        return Err(Error::Invalid(
            crate::domain::suite::constants::REQUIRED_FILE_OBSERVATIONS_ARE_MISSING.into(),
        ));
    }
    Ok(())
}
