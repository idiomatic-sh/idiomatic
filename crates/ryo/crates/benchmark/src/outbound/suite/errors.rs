//! The `suite` port's failures.

/// Why an operation of `suite` failed.
#[derive(Debug, thiserror::Error)]
pub enum SuiteError {
    /// Artifact IO failed.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// Authored wire data could not be decoded.
    #[error(transparent)]
    Decode(#[from] toml::de::Error),
    /// Artifact identity or boundary mismatch.
    #[error("{0}")]
    Invalid(String),
}
