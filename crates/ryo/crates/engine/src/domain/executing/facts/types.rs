//! The types of the `facts` part: the columns of the language's facts of a file, as a need's terms read them.

use crate::domain::code::types::{RelationNumber, Short};
use crate::domain::indexes::types::RowRun;
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::Count;

/// The language's facts of a file as columns: the rows of a relation the file holds whole, and a cell of a row. No
/// read through it fills: a relation not whole yet has no rows here.
pub(crate) trait FactColumns {
    /// The rows of the relation, when the file holds it whole; none before.
    fn whole(&self, relation: RelationNumber) -> Option<RowRun>;

    /// The cell of a row of the relation in the column at a place.
    fn cell(&self, relation: RelationNumber, column: Short, row: Count) -> CellWord;
}
