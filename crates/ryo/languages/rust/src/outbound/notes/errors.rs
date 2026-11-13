//! The `notes` port's failures.

/// Why an operation of `notes` failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum NotesError {
    /// The bytes are not exactly one note: none, cut short, or followed by more.
    #[error("the bytes are not exactly one note")]
    NotANote,
}

impl From<NotesError> for ryo::languages::ProjectError {
    /// Bytes the project reads as a file's note that hold none: the links cannot be drawn without losing that file's facts.
    fn from(error: NotesError) -> Self {
        Self::Context {
            reason: ryo::languages::ToolMessage(error.to_string().into()),
        }
    }
}
