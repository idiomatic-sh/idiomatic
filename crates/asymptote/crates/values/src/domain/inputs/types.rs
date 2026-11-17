//! The types of the `inputs` concept: plain data, public fields and derives, with no constructor and no inherent impl.

/// A path, or one segment of it, borrowed from the listing it is read from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Segment<'a>(pub &'a str);

/// Whether a path is one a pathspec's wildcards match.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Named {
    /// It is.
    Named,
    /// It is not.
    Unnamed,
}
