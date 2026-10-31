//! Measured wall time covers the complete child process, including startup.
use crate::domain::diagnostics::types::RunReport;
use std::time::Duration;

/// An individual process observation. No CPU or RSS estimate is fabricated.
#[derive(Clone, Debug)]
pub struct Measurement {
    /// Child startup-to-exit elapsed wall time, excluding output normalization.
    pub wall: Duration,
    /// Child CPU and resident memory, when collected by the execution boundary.
    pub resources: Option<Resources>,
    /// Decoded findings and the exact submitted file list.
    pub report: RunReport,
}

use std::ffi::OsString;
use std::path::PathBuf;
/// A direct process invocation without shell argument interpretation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Invocation {
    /// Program to launch directly.
    pub executable: PathBuf,
    /// Arguments passed to the program without shell expansion.
    pub arguments: Vec<OsString>,
    /// Explicit environment overrides, such as a pinned worker count.
    pub environment: std::collections::BTreeMap<OsString, OsString>,
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
    /// Explicitly absent for portable wall-only execution.
    pub resources: Option<Resources>,
}

/// OS observations for the child command, excluding the parent harness.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Resources {
    /// User plus system CPU consumed by the child and waited-for descendants.
    pub cpu: Duration,
    /// Maximum resident set reported by GNU time, converted from KiB to bytes.
    pub peak_rss_bytes: u64,
}

/// Reference and candidate observations retained after exact agreement.
#[derive(Clone, Debug)]
pub struct Pair {
    /// Observation produced by the pinned reference linter.
    pub reference: Measurement,
    /// Observation produced by the SDK candidate worker.
    pub candidate: Measurement,
}

/// Which contestant gets first access in a paired observation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunOrder {
    /// Observe the reference before the candidate.
    ReferenceFirst,
    /// Observe the candidate before the reference.
    CandidateFirst,
}

/// One named contestant observation retained only after correctness checks pass.
#[derive(Clone, Debug)]
pub struct Sample {
    /// Stable project/contestant/cache-condition identity in the BMF document.
    pub name: String,
    /// Full observation, including explicit resource availability.
    pub measurement: Measurement,
}

/// SHA-256 identity of an executable selected by the composition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutableDigest(pub String);
