//! The types of the `cells` part: where a branch's text cell is read from.

use crate::domain::lowering::flattening::types::FlatVariable;
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::{Column, Relation};

/// A text cell of a filled relation's row: the relation and the column a branch's variable is bound to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FilledCell {
    /// The relation the language fills.
    pub(crate) relation: Relation,
    /// The symbol column.
    pub(crate) column: Column,
}

/// What a branch holds of one element of the node table, gathered from its rows of node relations.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ShapeElement {
    /// Its kind's word, when a row holds it to a constant.
    pub(crate) kind: Option<CellWord>,
    /// Its field's word, when a row holds it to a constant.
    pub(crate) field: Option<CellWord>,
    /// The variable of its parent, when a row names one.
    pub(crate) parent: Option<FlatVariable>,
    /// Whether a relation of extent `Own` holds it.
    pub(crate) own: bool,
}
