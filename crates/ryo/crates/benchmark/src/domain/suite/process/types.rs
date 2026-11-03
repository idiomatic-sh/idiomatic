//! Immutable templates and independently archived process observations.
use crate::domain::suite::types::ArtifactDigest;
use std::path::PathBuf;

/// Encoding of an authoritative declared-input path within an output stream.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathEncoding {
    /// Exact UTF8 path spelling.
    Utf8Bytes,
    /// serde_json string content, without the enclosing quotes.
    JsonStringContent,
}
/// Exact literal bytes or one explicitly declared input path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OutputPart {
    /// Every byte, including whitespace, remains significant.
    Literal(String),
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
    pub exit: i32,
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
    Diagnostics(crate::domain::diagnostics::types::RunReport),
    /// A separate process observation, never an empty diagnostic report.
    ProcessFailure(ProcessFailure),
}

/// An offline audit keeps valid pairs available for safe shard resume without hiding failures.
#[derive(Clone, Debug, Default)]
pub struct ReceiptAudit {
    /// Every registered case, including missing or invalid observations.
    pub case_count: usize,
    /// Exact identities that independently passed the shared validator.
    pub verified: Vec<(String, String)>,
    /// Metadata contracts independently recomputed by this verifier, never echoed from pass flags.
    pub metadata_verified: Vec<(String, String, super::super::types::ArtifactDigest)>,
    /// Strict failures, never skipped successes.
    pub errors: Vec<String>,
    /// Raw report equality, excluding process observations.
    pub raw_exact: usize,
    /// Documented diagnostic or process discrepancies.
    pub documented_discrepancies: usize,
    /// Named documented tool policy differences, a subset of documented discrepancies.
    pub tool_contract_differences: Vec<(String, String)>,
    /// Explicit diagnostics-contract-only equality.
    pub contract_only: usize,
}
