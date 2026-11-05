//! Artifact identity and its existing display vocabulary.
/// Stable digest of the complete requested artifact set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactDigest(
    /// Framed artifact fingerprint, or raw executable hash when explicitly requested.
    pub String,
);

impl ArtifactDigest {
    /// Bind every declared path and execute-bit truth state with unambiguous framing.
    /// Unspecified observations retain their distinct absent identity.
    ///
    /// # Errors
    /// Refuses duplicate/unordered paths or a length outside the canonical framing.
    pub fn file_metadata<'a>(
        metadata: Option<impl IntoIterator<Item = super::super::FileMetadataEntry<'a>>>,
    ) -> Result<Option<Self>, crate::domain::reports::errors::ReportsError> {
        use sha2::{Digest as _, Sha256};
        metadata
            .map(|values| {
                let mut hash = Sha256::new();
                hash.update(crate::domain::artifacts::constants::METADATA_HEADER);
                let mut previous = None;
                for entry in values {
                    if previous.is_some_and(|path| path >= entry.path) {
                        return Err(crate::domain::reports::errors::ReportsError::Invalid(
                            crate::domain::artifacts::constants::METADATA_ORDER.into(),
                        ));
                    }
                    previous = Some(entry.path);
                    let bytes = entry.path.0.as_bytes();
                    hash.update(
                        u64::try_from(bytes.len())
                            .map_err(|_| {
                                crate::domain::reports::errors::ReportsError::Invalid(
                                    crate::domain::artifacts::constants::METADATA_PATH_LENGTH
                                        .into(),
                                )
                            })?
                            .to_le_bytes(),
                    );
                    hash.update(bytes);
                    hash.update([u8::from(
                        entry.executable == super::super::FileExecutable::Set,
                    )]);
                }
                Ok(Self(format!("{:x}", hash.finalize())))
            })
            .transpose()
    }
}

impl std::fmt::Display for ArtifactDigest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
