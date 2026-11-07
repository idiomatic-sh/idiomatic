//! The types of the `imports` concept: plain data, public fields and derives, with no constructor
//! and no inherent impl.

use ryo::languages;

/// What a `use` tree names so far: the path down to it, and the last name on it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Imported {
    /// The path, its segments joined with `::`.
    pub target: languages::Symbol,
    /// The last segment of the path, which is the name it binds unless it is renamed.
    pub last: languages::Symbol,
}

/// Whether a path of a `use` starts at the root of the crates, with `::`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Rooting {
    /// It does: `use ::std;`.
    Absolute,
    /// It does not.
    Relative,
}

/// How the path of a `use` ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Ending {
    /// On a name.
    Name,
    /// On a glob.
    Glob,
}

/// What a leaf of a `use` tree brings in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Import {
    /// The name it binds.
    pub binding: languages::Symbol,
    /// The path it imports, its segments joined with `::`.
    pub target: languages::Symbol,
}
