//! The types of the `patterns` concept: plain data, public fields and derives, with no constructor and no inherent impl.

/// A directory, relative to the root of the repository; empty for the root.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Directory<'a>(pub &'a str);

/// A part of a path or a pattern.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PatternPart<'a>(pub &'a str);

/// A pattern of directories: parts separated by `/`, in which `*` is any run of characters inside a part, `?` any one, and a part `**`
/// any number of parts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Pattern(pub Box<str>);

/// Whether a directory is the one a pattern names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Matching {
    /// It is.
    Matches,
    /// It is not.
    Differs,
}
