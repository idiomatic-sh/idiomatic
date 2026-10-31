//! Boundary and validation failures never turn into empty reports.
/// Errors always prevent a correctness or performance result.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A benchmark input, tool result or comparison violates its required contract.
    #[error("{0}")]
    Invalid(String),
    /// Reading inputs, preparing workspaces or executing a process failed.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// A process-boundary JSON message could not be decoded or encoded.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// A benchmark artifact could not be decoded from TOML.
    #[error(transparent)]
    Toml(#[from] toml::de::Error),
}
