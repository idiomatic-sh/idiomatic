//! Metadata validation is independent of dependency projection and performs no filesystem IO.
use crate::domain::packages::types::Reason;
use ryo::languages::Span;
/// Which metadata result the caller requested from an existing manifest read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MetadataDemand {
    /// Dependency projection alone; do not perform additional metadata validation.
    Projection,
    /// Standard project-metadata diagnostics alone; do not project dependencies.
    Diagnostics,
    /// Both independent outputs from the same parsed document.
    Both,
}
/// The stage that produced a metadata diagnostic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Phase {
    /// The source is not valid TOML.
    Parse,
    /// TOML parsed, but a metadata schema or typed value was rejected.
    Validation,
}
/// A location supplied by the actual parser/schema error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Location {
    /// An actual source byte range retained by the deserializer.
    Span(Span),
    /// The error genuinely supplied no finer location; this means file origin, not guessed offsets.
    FileOrigin,
}
/// One metadata diagnostic and its original detail.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ManifestDiagnostic {
    /// Parsing and metadata failures remain distinguishable.
    pub phase: Phase,
    /// Source mapping, without fabricated AST nodes.
    pub location: Location,
    /// Original parser or schema diagnostic detail.
    pub message: Reason,
}
/// A requested validator's observed diagnostics, independently of dependency projection.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Validation {
    /// Actual diagnostics, including syntax rejection before metadata validation could run.
    pub diagnostics: Vec<ManifestDiagnostic>,
    /// A validator/mapping failure makes absence of further diagnostics unknown.
    pub incomplete: Option<Reason>,
}

/// Validation for a real requested manifest file, numbered by the project request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FileValidation {
    /// Existing requested file identity, never an invented manifest index.
    pub file: ryo::languages::FileId,
    /// Independent validation result.
    pub validation: Validation,
}

/// Complete primary validation results and explicit unknown validation work.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ManifestValidations {
    /// Primary files for which the validator ran.
    pub files: Vec<FileValidation>,
    /// Missing validators or unrepresentable file identities cannot imply known absence.
    pub incomplete: Vec<Reason>,
}
