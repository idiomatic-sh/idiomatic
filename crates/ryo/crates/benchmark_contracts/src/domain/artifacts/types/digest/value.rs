//! Artifact identity and its existing display vocabulary.
/// Stable digest of the complete requested artifact set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactDigest(
    /// Framed artifact fingerprint, or raw executable hash when explicitly requested.
    pub String,
);

impl std::fmt::Display for ArtifactDigest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
