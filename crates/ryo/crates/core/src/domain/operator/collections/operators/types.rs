//! The types of the `operators` collection: plain data, public fields and derives, with no constructor and no inherent impl.

/// The pattern of a capture, written with slots: `docs/{page}.md`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SlotPattern<'a>(pub &'a str);

/// The whole text a capture reads, which the ranges of its slots are of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WholePath<'a>(pub &'a str);

/// The end of the text a capture reads that the rest of the pattern is still to match.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PathTail<'a>(pub &'a str);
