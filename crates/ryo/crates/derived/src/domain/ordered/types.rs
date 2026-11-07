//! The types of the `ordered` concept: plain data, public fields and derives, with no constructor and no inherent impl.
use ryo_relations::domain::relations::types::{Integer, Value};
use ryo_values::domain::rows::types::RowIndex;

/// The text of an ordered key.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct OrderedText(pub String);

/// One cell of an ordered key: text, or the cell itself.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum OwnedOrderedKey {
    /// A symbol or name resolved to its text.
    Text(OrderedText),
    /// Any other cell.
    Value(Value),
}

/// One row with its owned key and its order number.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct OwnedOrderedRow {
    /// The key cells.
    pub keys: Vec<OwnedOrderedKey>,
    /// The order number within the key.
    pub order: Integer,
    /// The row.
    pub row: RowIndex,
}

/// Both sides of an ordered matching, each sorted by key then order.
#[derive(Debug)]
pub struct OrderedMatching {
    /// The left side.
    pub left: Vec<OwnedOrderedRow>,
    /// The right side.
    pub right: Vec<OwnedOrderedRow>,
}
