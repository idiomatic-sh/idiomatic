//! Measured wall time covers the complete child process, including startup.
use crate::domain::reports::types::RunReport;
use std::time::Duration;

/// An individual process observation. No CPU or RSS estimate is fabricated.
#[derive(Clone, Debug)]
pub struct Measurement {
    /// Child startup-to-exit elapsed wall time, excluding output normalization.
    pub wall: Duration,
    /// Child CPU and resident memory, when collected by the execution boundary.
    pub resources: ResourceObservation,
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
    /// Unix terminating signal, absent for numeric exits and non-Unix platforms.
    pub signal: Option<i32>,
    /// Numeric process exit status.
    pub status: ProcessExitCode,
    /// Raw bytes captured from standard output.
    pub stdout: CapturedBytes,
    /// Raw bytes captured from standard error.
    pub stderr: CapturedBytes,
    /// Elapsed wall time covering process startup, execution and waiting.
    pub wall: Duration,
    /// Explicitly absent for portable wall-only execution.
    pub resources: ResourceObservation,
}

/// OS observations for the child command, excluding the parent harness.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Resources {
    /// User plus system CPU consumed by the child and waited-for descendants.
    pub cpu: Duration,
    /// Maximum resident set from the direct child collector, expressed in bytes.
    pub peak_rss_bytes: ResidentBytes,
}

/// Which contestant gets first access in a paired observation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunOrder {
    /// Observe the reference before the candidate.
    ReferenceFirst,
    /// Observe the candidate before the reference.
    CandidateFirst,
}

/// Exit code returned by a successfully launched process.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProcessExitCode(
    /// Exit status reported by the operating system.
    pub i32,
);

/// Captured process stream bytes, not source text or a machine path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapturedBytes(
    /// Unmodified bytes captured from one process stream.
    pub Vec<u8>,
);

/// Peak resident memory expressed in bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResidentBytes(
    /// Peak resident memory in bytes.
    pub u64,
);

/// Stable identity of a project, contestant and cache condition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SampleName(
    /// Project, contestant and cache-condition identity.
    pub String,
);

/// Whether the execution boundary actually collected child resources.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceObservation {
    /// Portable execution measured wall time only.
    WallOnly,
    /// CPU and peak memory came from the owned child process.
    Collected(Resources),
}

/// Exact execution mode and instrumentation configuration bound by certification.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExecutionProfile {
    /// Portable process execution records elapsed wall time only.
    Wall,
    /// Native resource collection in the spawning process.
    Direct,
    /// A separately executed, pinned collector owns contestant launch.
    Isolated(CollectorProfile),
}

/// Identity of the collector executable and its exact byte pin.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CollectorProfile {
    /// Actual collector program passed to process spawning.
    pub executable: PathBuf,
    /// Unframed SHA256 of that program's bytes.
    pub digest: CollectorDigest,
}

/// Raw SHA256 identifying an instrumentation executable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CollectorDigest(
    /// Expected SHA256 hexadecimal digest of collector bytes.
    pub String,
);

/// Whether a timed invocation is a complete benchmark or an explicitly narrowed diagnostic.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MeasurementMode {
    /// Immutable declared rule identities captured before selection/projection.
    Benchmark { inventory: Vec<ryo::linter::RuleId> },
    /// Scoped observations are diagnostic evidence, never a complete benchmark.
    Diagnostic,
}

/// Borrowed output data projected from an application's checked measurement sample.
#[derive(Clone, Copy, Debug)]
pub struct PublicationSample<'a> {
    /// Explicit complete-inventory status; a diagnostic cannot become a metric.
    pub complete_inventory: bool,
    /// Stable project, contestant and cache-condition identity.
    pub name: &'a SampleName,
    /// Original observation; output adapters must not clone or replace its report.
    pub measurement: &'a Measurement,
}
