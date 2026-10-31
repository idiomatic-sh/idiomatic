//! Measured wall time covers the complete child process, including startup.
use crate::domain::diagnostics::types::RunReport;
use std::time::Duration;

/// An individual process observation. No CPU or RSS estimate is fabricated.
#[derive(Clone, Debug)]
pub struct Measurement {
    /// Child startup-to-exit elapsed wall time, excluding output normalization.
    pub wall: Duration,
    /// Decoded findings and the exact submitted file list.
    pub report: RunReport,
}

use std::ffi::OsString;
use std::path::PathBuf;
/// A direct process invocation without shell argument interpretation.
#[derive(Clone, Debug)]
pub struct Invocation {
    /// Program to launch directly.
    pub executable: PathBuf,
    /// Arguments passed to the program without shell expansion.
    pub arguments: Vec<OsString>,
    /// Working directory of the child process.
    pub directory: PathBuf,
}

/// Captured child exit status, byte streams and elapsed wall time.
#[derive(Clone, Debug)]
pub struct ProcessOutput {
    /// Numeric process exit status.
    pub status: i32,
    /// Raw bytes captured from standard output.
    pub stdout: Vec<u8>,
    /// Raw bytes captured from standard error.
    pub stderr: Vec<u8>,
    /// Elapsed wall time covering process startup, execution and waiting.
    pub wall: Duration,
}

/// Reference and candidate observations retained after exact agreement.
#[derive(Clone, Debug)]
pub struct Pair {
    /// Observation produced by the pinned reference linter.
    pub reference: Measurement,
    /// Observation produced by the SDK candidate worker.
    pub candidate: Measurement,
}
