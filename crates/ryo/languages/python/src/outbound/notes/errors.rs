//! The `notes` port's failures.

/// Why an operation of `notes` failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum NotesError {
    /// The bytes are not exactly one note: none, cut short, or followed by more.
    #[error("the bytes are not exactly one note")]
    NotANote,
}
