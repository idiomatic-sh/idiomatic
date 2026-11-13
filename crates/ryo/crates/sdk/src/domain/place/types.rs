//! The SDK's `place` concept: where in its file a finding is.

/// A line of a file, counting from one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Line(pub u32);

/// A character of a line, counting from one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Column(pub u32);

/// Where in its file a finding is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Place {
    /// The finding has no place in the file: it is of the file, or of several.
    Missing,
    /// The finding starts here.
    At {
        /// The line.
        line: Line,
        /// The character in the line.
        column: Column,
    },
}
