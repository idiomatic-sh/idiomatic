//! The types of the `batch` concept. Plain data, public fields and derives.

use ryo_relations::domain::relations::types::Value;

/// The rows of one step for one file: `count` rows of `width` cells each, one after another.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Rows {
    /// How many cells a row has.
    pub width: usize,
    /// How many rows there are.
    pub count: usize,
    /// The cells, row after row.
    pub cells: Vec<Value>,
}
