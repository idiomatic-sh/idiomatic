//! The types of the `constraints` concept: plain data, public fields and derives, with no constructor and no inherent impl.

/// The bytes of a file, or of one of its lines, as they are written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SourceBytes<'a>(pub &'a [u8]);

/// Where a comment starts in its file's bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct At(pub usize);

/// Which build constraint a comment is, if it is one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Constraint {
    /// A `//go:build` line in the file's header.
    GoBuild,
    /// A `// +build` line in the file's header, which a blank line follows before the first line that is no comment.
    PlusBuild,
    /// Any other comment, or a constraint line where `go/build` does not read one.
    Other,
}

/// Whether what follows the word of a constraint ends the word.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Ending {
    /// Nothing follows it, or white space does: `//go:build linux`.
    Ends,
    /// Another letter does: `//go:buildx`.
    Continues,
}

/// Whether a block comment is open where a line is read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BlockComment {
    /// A `/*` has no `*/` yet.
    Open,
    /// None is.
    Closed,
}

/// What a line of the header holds once its comments are left out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Content {
    /// Text that is no comment: the header ends there.
    Code,
    /// Comments and white space alone.
    Comments,
}
