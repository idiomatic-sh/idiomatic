//! Immutable templates and independently archived process observations.
use crate::domain::artifacts::types::digest::value::ArtifactDigest;
use crate::domain::measurements::types::ProcessExitCode;
use std::path::PathBuf;

/// Encoding of an authoritative declared-input path within an output stream.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathEncoding {
    /// Exact UTF8 path spelling.
    Utf8Bytes,
    /// serde_json string content, without the enclosing quotes.
    JsonStringContent,
}
/// Exact literal text of an output stream.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutputText(
    /// Every byte of the literal, including whitespace.
    pub String,
);

/// Exact literal bytes or one explicitly declared input path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OutputPart {
    /// Every byte, including whitespace, remains significant.
    Literal(OutputText),
    /// The path must name one actual requested source file.
    InputPath {
        file: PathBuf,
        encoding: PathEncoding,
    },
}
/// A safe profile-relative raw artifact and its exact byte identity.
#[derive(Clone, Debug)]
pub struct ProcessArtifact {
    /// Profile-relative file, never an implicit external read.
    pub path: PathBuf,
    /// SHA256 of unmodified bytes.
    pub sha256: ArtifactDigest,
}
/// One archived invocation; two are required for each exact executable group.
#[derive(Clone, Debug)]
pub struct ProcessObservation {
    /// Exact binary; another approved architecture is not equivalent evidence.
    pub executable_sha256: ArtifactDigest,
    /// Root/argv/environment/status and normalized settings captured independently.
    pub invocation: ProcessArtifact,
    /// Raw captured standard output.
    pub stdout: ProcessArtifact,
    /// Raw captured standard error.
    pub stderr: ProcessArtifact,
}
/// An expected completed numeric failure, never a timeout, signal or spawn failure.
#[derive(Clone, Debug)]
pub struct ProcessFailure {
    /// Exact nonzero numeric process status.
    pub exit: ProcessExitCode,
    /// Whole standard output, with only declared path slots variable.
    pub stdout: Vec<OutputPart>,
    /// Whole standard error, with only declared path slots variable.
    pub stderr: Vec<OutputPart>,
    /// At least two independently captured observations per supported binary.
    pub observations: Vec<ProcessObservation>,
}

/// The reference either returns a complete report or one explicitly evidenced process failure.
#[derive(Clone, Debug)]
pub enum ReferenceExpectation {
    /// Existing complete diagnostics expectation.
    Diagnostics(crate::domain::reports::types::RunReport),
    /// A separate process observation, never an empty diagnostic report.
    ProcessFailure(ProcessFailure),
}
