//! Artifact identities and addresses that cannot be formed never turn into a default value.

/// Errors always prevent an artifact identity or address from being formed.
#[derive(Debug, thiserror::Error)]
pub enum ArtifactsError {
    /// A declared artifact path or metadata inventory violates its canonical form.
    #[error("{0}")]
    Invalid(String),
}
