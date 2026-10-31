//! Explicit identity of the instrumentation executable.
use std::path::PathBuf;

/// GNU time must match this executable digest before any contestant is launched.
#[derive(Clone, Debug)]
pub struct ExecutionTimedSettings {
    /// Absolute executable path, never interpreted by a shell.
    pub executable: PathBuf,
    /// SHA-256 of the executable bytes approved for this run.
    pub sha256: String,
}
