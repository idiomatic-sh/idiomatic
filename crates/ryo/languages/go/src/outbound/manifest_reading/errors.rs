//! The errors of the `manifest_reading` port.

/// Why a manifest could not be read.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum ManifestReadingError {
    /// A `go.mod` with no `module` directive.
    #[error("the go.mod has no module directive")]
    NoModule,
    /// A block closed that is not open.
    #[error("line {line}: `)` closes no block")]
    Unopened {
        /// The line.
        line: usize,
    },
    /// A block that is not closed.
    #[error("line {line}: the `{verb} (` block is not closed")]
    Unclosed {
        /// The line it opens at.
        line: usize,
        /// The directive of the block.
        verb: String,
    },
    /// A directive with too few arguments.
    #[error("line {line}: `{verb}` is missing arguments")]
    Incomplete {
        /// The line.
        line: usize,
        /// The directive.
        verb: String,
    },
    /// A string that is not closed.
    #[error("line {line}: a quoted string is not closed")]
    Unquoted {
        /// The line.
        line: usize,
    },
}
