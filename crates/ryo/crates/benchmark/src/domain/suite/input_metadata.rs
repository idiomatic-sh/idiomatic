//! Pure identity of declared file observations, separate from unchanged source-byte framing.
use super::types::ArtifactDigest;
use crate::domain::diagnostics::errors::DiagnosticsError as Error;
use ryo::linter::RelativePath;
use sha2::{Digest as _, Sha256};
use std::collections::BTreeMap;

/// None is the historical unspecified case; an explicitly empty table has its own identity.
pub fn digest(
    metadata: Option<&BTreeMap<RelativePath, bool>>,
) -> Result<Option<ArtifactDigest>, Error> {
    metadata
        .map(|values| {
            let mut hash = Sha256::new();
            hash.update(b"ryo-case-file-metadata-v1\0");
            for (path, executable) in values {
                let bytes = path.0.as_bytes();
                hash.update(
                    u64::try_from(bytes.len())
                        .map_err(|_| {
                            Error::Invalid("metadata path length exceeds canonical encoding".into())
                        })?
                        .to_le_bytes(),
                );
                hash.update(bytes);
                hash.update([u8::from(*executable)]);
            }
            Ok(ArtifactDigest(format!("{:x}", hash.finalize())))
        })
        .transpose()
}

/// A required observation must be explicit for every declared source, even for negative cases.
pub fn require(
    requirement: Option<super::types::FileObservation>,
    files: &[RelativePath],
    metadata: Option<&BTreeMap<RelativePath, bool>>,
) -> Result<(), Error> {
    if let Some(values) = metadata {
        for path in values.keys() {
            super::service::relative_path(std::path::Path::new(path.0.as_ref()))?;
            if !files.contains(path) {
                return Err(Error::Invalid("metadata names an undeclared source".into()));
            }
        }
    }
    if requirement.is_some()
        && metadata.is_none_or(|values| {
            values.len() != files.len() || files.iter().any(|file| !values.contains_key(file))
        })
    {
        return Err(Error::Invalid(
            "required file observations are missing".into(),
        ));
    }
    Ok(())
}
