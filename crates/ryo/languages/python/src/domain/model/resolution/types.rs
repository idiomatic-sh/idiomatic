//! The types of the model's `resolution` part: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo::languages::Position;

/// Whether a definition is in effect where a name is read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Bound {
    /// It is: the read is outside the statement that is still evaluating the definition's value.
    Yes,
    /// It is not yet.
    NotYet,
}

/// Whether the origin of a name that is read is proven.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OriginProof {
    /// Nothing between the definition and the read can have changed it.
    Proven,
    /// A wildcard import or a deletion can have changed it.
    Unproven,
}

/// The resolution part of the model, cleared between files.
#[derive(Clone, Debug, Default)]
pub(crate) struct ResolutionModel {
    /// The definition each name expression that is read resolves to; the position of no node when it resolves to none.
    pub resolved: Vec<Position>,
}
