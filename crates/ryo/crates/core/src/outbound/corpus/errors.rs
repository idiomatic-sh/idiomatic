//! The `corpus` port's failures.

/// Why the `corpus` port failed.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum CorpusError {
    /// The root, or something below it, could not be read.
    #[error("cannot read `{path}`: {reason}")]
    Unreadable {
        /// The root asked for, or the file below it that could not be read.
        path: String,
        /// What went wrong.
        reason: String,
    },
    /// The file changed while it was read, twice: what was read is not any version of it, so it is not analysed or kept.
    #[error("`{path}` changed while it was read")]
    Unstable {
        /// The file's path, relative to the root.
        path: String,
    },
    /// The file is larger than the engine can number: a span is stored in four bytes.
    #[error("`{path}` is {} bytes, more than the four-byte offsets of the engine can name", size.0)]
    TooLarge {
        /// The file's path, relative to the root.
        path: String,
        /// Its size in bytes.
        size: ryo_values::domain::files::types::FileSize,
    },
}
