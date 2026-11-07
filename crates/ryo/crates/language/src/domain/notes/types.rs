//! The types of the `notes` concept: whether a reader noted any word.

/// Whether a reader has noted any word while reading a file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Noted {
    /// It has.
    Words,
    /// It has not.
    Nothing,
}
