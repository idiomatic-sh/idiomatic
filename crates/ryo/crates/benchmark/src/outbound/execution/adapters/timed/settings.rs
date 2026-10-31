//! Explicit identity of the instrumentation executable.
use crate::domain::measurements::types::ExecutableDigest;
use std::{ffi::OsString, path::PathBuf};

/// GNU time must match this executable digest before any contestant is launched.
#[derive(Clone, Debug)]
pub(crate) struct ExecutionTimedSettings {
    /// Absolute executable path, never interpreted by a shell.
    pub executable: PathBuf,
    /// SHA-256 of the executable bytes approved for this run.
    pub sha256: ExecutableDigest,
    /// Inherited executable lookup path captured once by the composition.
    pub(crate) path: Option<OsString>,
}
