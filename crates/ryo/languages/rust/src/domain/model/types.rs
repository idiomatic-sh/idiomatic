//! The types of the `model` concept: plain data, public fields and derives, with no constructor and no inherent impl. What the
//! language's readers work out of a file once, on the first read that asks.

use ryo::languages::ShapeIndex;

/// The indexes the readers of a file build on demand; each is emptied for the next file and keeps its capacity.
#[derive(Debug, Default)]
pub(crate) struct ReadIndexes {
    /// The structural classes of the file's nodes, built on the first `expression_shape` read.
    pub shapes: ShapeIndex,
    /// Grammar under which this file was read.
    pub revision: ProviderRevision,
}

/// The language facts published by the provider revision selected at the composition root.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum ProviderRevision {
    /// Standard expression macros and literal expressions in the released 0.1.4 contract.
    Released014,
    /// Macro arguments, attributes and ABI strings in the current contract.
    #[default]
    Current,
}
