//! The types of the `attribute_paths` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo::languages::Position;

/// Where a path an attribute writes lies: the element it starts at and the element it ends at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PathExtent {
    /// The path node of a meta, or the first token (a segment or the first `:` of a leading `::`) of a path written in a token tree.
    pub(crate) first: Position,
    /// The path node itself, or the last segment token of a path written in a token tree.
    pub(crate) last: Position,
}

/// Whether a path reads an element: whitespace and comments are skipped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Significance {
    /// Whitespace or a comment.
    Skipped,
    /// Any other element.
    Read,
}

/// Whether a token can be one segment of a path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Segmenthood {
    /// An identifier, `self`, `super`, `crate` or `Self`.
    Segment,
    /// Any other token.
    Other,
}
