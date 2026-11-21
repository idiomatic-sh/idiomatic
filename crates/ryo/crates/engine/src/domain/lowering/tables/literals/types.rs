//! The types of the `literals` part.

use crate::domain::lowering::flattening::types::FlatTerm;
use ryo_values::domain::plan_parts::types::Side;

/// What a text a comparison holds a column to is as a cell of that column.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum LiteralCell {
    /// The column holds the cell this one term is: a flag's stored bits, a closed set's word.
    Exactly(FlatTerm),
    /// What the column holds is not known from the plan (a symbol column that is no column of a relation any more), and the
    /// text is also a word of a closed set: the column holds one of these, the text's own word first.
    Either(Vec<FlatTerm>),
    /// No cell of the column is shown as the text: an equality holds of no row, an inequality of every row.
    Never,
}

/// A text a comparison holds a column to: which side of the comparison the text is on, and what it is as the column's cell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct HeldLiteral {
    /// The side the text is on.
    pub(crate) side: Side,
    /// The text as the column's cell.
    pub(crate) cell: LiteralCell,
}
