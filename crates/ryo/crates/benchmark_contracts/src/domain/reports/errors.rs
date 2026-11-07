//! Boundary and validation failures never turn into empty reports.
/// Errors always prevent a correctness or performance result.
#[derive(Debug, thiserror::Error)]
pub enum ReportsError {
    /// A benchmark input, tool result or comparison violates its required contract.
    #[error("{0}")]
    Invalid(String),
    /// Reading inputs, preparing workspaces or executing a process failed.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// A process-boundary JSON message could not be decoded or encoded.
    #[error(transparent)]
    Json(Box<dyn std::error::Error + Send + Sync>),
    /// A benchmark artifact could not be decoded from TOML.
    #[error(transparent)]
    Toml(#[from] toml::de::Error),
}

impl From<crate::domain::artifacts::errors::ArtifactsError> for ReportsError {
    fn from(error: crate::domain::artifacts::errors::ArtifactsError) -> Self {
        match error {
            crate::domain::artifacts::errors::ArtifactsError::Invalid(reason) => Self::Invalid(reason),
        }
    }
}
