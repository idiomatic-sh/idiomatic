//! The types of the `walks` part: a walk's two ends, and the tree a nearest walk climbs.

use crate::domain::lowering::flattening::types::{FlatTerm, NearestOver};
use crate::domain::steps::types::{DefinitionId, MadeColumn};

/// A nearest walk: what is walked toward the root, and the column of the definition that holds its domain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NearestClimb {
    /// What is walked toward the root.
    pub(crate) over: NearestOver,
    /// The definition of the domain.
    pub(crate) domain: DefinitionId,
    /// The domain's column that holds the element.
    pub(crate) column: MadeColumn,
}

/// The two ends of a walk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WalkEnds {
    /// The element walked from.
    pub(crate) start: FlatTerm,
    /// The element reached.
    pub(crate) reached: FlatTerm,
}
