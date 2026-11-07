//! The types of the `shapes` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::grammar::types::Index;

/// Whether a node of the library speaks for itself, or the list it is in has already found its node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Silence {
    /// It finds its own nodes.
    Speaks,
    /// The list it is in found the field it is, and it finds nothing for itself.
    Quiet,
}

/// What to do after a node of the library: leave it, or go on to the nodes under it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Descent {
    /// There is nothing to find under it.
    Stop,
    /// Go on to the nodes under it.
    Below,
    /// Go on to the nodes under it, whose declarations of parameters find nothing for themselves.
    Fieldless,
}

/// A node of the library's tree still to be visited.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Visit {
    /// The node.
    pub at: Index,
    /// Whether it finds its own nodes.
    pub silence: Silence,
}
