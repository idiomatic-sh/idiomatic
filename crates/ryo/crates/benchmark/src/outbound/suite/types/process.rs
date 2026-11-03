//! Serialized process expectations; acceptance belongs to shared application/domain policy.
use serde::Deserialize;
use std::path::PathBuf;

/// Only this explicit variant can request process-failure comparison.
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ProcessFailureWire {
    /// A completed numeric failure with exactly pinned byte streams.
    ProcessFailure {
        /// Exact nonzero numeric status, never a signal.
        exit: i32,
        /// Whole output template.
        stdout: Vec<OutputPartWire>,
        /// Whole error template.
        stderr: Vec<OutputPartWire>,
        /// Two captures for each exact supported executable.
        observations: Vec<ProcessObservationWire>,
    },
}
/// Exact byte spelling or a declared source path in one explicit encoding.
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum OutputPartWire {
    /// Unmodified UTF8 bytes, including whitespace.
    Literal { value: String },
    /// Authoritative invocation root joined to a declared input.
    InputPath {
        file: PathBuf,
        encoding: PathEncodingWire,
    },
}
/// Output path encoding, without inferred normalization.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PathEncodingWire {
    /// Exact UTF8 bytes.
    Utf8Bytes,
    /// Canonical JSON string content, without surrounding quotes.
    JsonStringContent,
}
/// A raw file stored separately from independent semantic evidence.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ProcessArtifactWire {
    /// Explicit profile-relative path.
    pub path: PathBuf,
    /// Exact raw byte SHA256.
    pub sha256: String,
}
/// Independent captured observation grouped by exact executable digest.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ProcessObservationWire {
    /// No cross-architecture substitution is accepted.
    pub executable_sha256: String,
    /// Captured root/normalized settings/exit/source identity.
    pub invocation: ProcessArtifactWire,
    /// Actual raw standard output.
    pub stdout: ProcessArtifactWire,
    /// Actual raw standard error.
    pub stderr: ProcessArtifactWire,
}

/// Captured verification-only bytes to persist beside one receipt file.
#[derive(Clone, Copy, Debug)]
pub struct CapturedProcess<'a> {
    /// Collision-safe key framed from rule/case/side by the application.
    pub key: &'a str,
    /// Actual root/files/normalized settings/executable/status provenance.
    pub invocation: &'a serde_json::Value,
    /// Exact unmodified stdout.
    pub stdout: &'a [u8],
    /// Exact unmodified stderr.
    pub stderr: &'a [u8],
}
