//! The types of the `views` part.

use crate::domain::lowering::flattening::types::FlatTerm;
use ryo_relations::domain::relations::types::Relation;

/// One read of a relation that a step's branches are: the relation, the cells of its row in the relation's column order,
/// and the step's own row, each column of it a cell of that row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RelationView {
    /// The relation.
    pub(crate) relation: Relation,
    /// The cells of the relation's row.
    pub(crate) cells: Vec<FlatTerm>,
    /// The step's row.
    pub(crate) row: Vec<FlatTerm>,
}
