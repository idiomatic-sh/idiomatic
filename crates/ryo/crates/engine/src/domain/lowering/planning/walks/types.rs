//! The types of the `walks` part: a walk's two ends, and the tree a nearest walk climbs.

use crate::domain::lowering::flattening::types::FlatTerm;
use crate::domain::steps::types::{DefinitionId, MadeColumn};
use ryo_relations::domain::relations::types::Relation;

/// A nearest walk: the node relation whose tree is walked, and the column of the definition that holds its domain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct NearestTreeWalk {
    /// The node relation.
    pub(crate) relation: Relation,
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
