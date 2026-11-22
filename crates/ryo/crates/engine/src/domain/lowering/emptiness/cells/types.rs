//! The types of the `cells` part: where a branch's text cell is read from.

use ryo_relations::domain::relations::types::{Column, Relation};

/// A text cell of a filled relation's row: the relation and the column a branch's variable is bound to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FilledCell {
    /// The relation the language fills.
    pub(crate) relation: Relation,
    /// The symbol column.
    pub(crate) column: Column,
}
