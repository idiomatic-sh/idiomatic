//! Why a registered case's input identities are refused.
use super::types::RefusalText;

/// A refused registered input declaration.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum RegisteredInputsError {
    /// Refuses: registered files are missing.
    #[error("registered files are missing")]
    RegisteredFilesMissing,
    /// Refuses: unsupported registered file observation.
    #[error("unsupported registered file observation")]
    UnsupportedFileObservation,
    /// Refuses: file metadata must be an object.
    #[error("file metadata must be an object")]
    FileMetadataNotObject,
    /// Refuses: metadata names an undeclared source.
    #[error("metadata names an undeclared source")]
    UndeclaredMetadataSource,
    /// Refuses: executable observation must be boolean.
    #[error("executable observation must be boolean")]
    ExecutableNotBoolean,
    /// Refuses: required input observations are missing.
    #[error("required input observations are missing")]
    InputObservationsMissing,
    /// Refuses: project search roots must be an ordered array.
    #[error("project search roots must be an ordered array")]
    SearchRootsNotArray,
    /// Refuses: project search root is not text.
    #[error("project search root is not text")]
    SearchRootNotText,
    /// A library refused the declaration with these words.
    #[error("{}", .0.0)]
    Refused(RefusalText),
}
