//! The types of the `paths` concept: plain data, public fields and derives, with no constructor and no inherent impl.

/// A path relative to the root of the repository, its segments joined by `/`; the root itself is the empty path.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RepoPath(pub Box<str>);

/// Whether a path lies at or beneath a directory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Containment {
    /// It is the directory or lies beneath it.
    Inside,
    /// It lies elsewhere.
    Outside,
}

/// One segment of a path: a directory or a file name.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Segment(pub Box<str>);

/// A path as it is written, before it is read as a path of the repository: separators of either kind, `.` and `..`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PathText<'a>(pub &'a str);

/// A path relative to a directory, as text: the empty text for the directory itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Relative<'a>(pub &'a str);
