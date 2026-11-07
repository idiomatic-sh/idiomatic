//! The types of the `tree_join` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo_relations::domain::relations::types::{Position, Value};

use crate::domain::kind_sets::types::KindScratch;
use crate::domain::operating::types::MissingColumn;
use crate::domain::operating::types::NameCount;
use crate::domain::table_cells::types::ColumnPlaces;
use crate::domain::table_scratch::types::{Candidates, ComparisonNumbers, TextVerdicts};

/// Whether a comparison of the filter reads a column the elements lack.
#[derive(Clone, Copy, Debug)]
pub(crate) enum ColumnLack {
    /// Every column the comparisons read is there.
    Nothing,
    /// A column is read that the elements lack.
    Column(MissingColumn),
}

/// Which of the elements the row on the left found the join reads next.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct StretchCursor(pub usize);

/// What a join looking rows up in a tree keeps while it reads the rows of its left input, one at a time: the room of its comparisons
/// and its answers, taken from the scratch and given back.
pub(crate) struct TreeState {
    /// How many names the symbols hold.
    pub names: NameCount,
    /// The row being answered.
    pub other: Vec<Value>,
    /// Reused left key buffer for the scheduled Join capability.
    pub key: Vec<Value>,
    /// The columns of the elements the comparisons read.
    pub needed: ColumnPlaces,
    /// What the kind sets decide.
    pub kinds: KindScratch,
    /// The comparisons the text of an element is told by first.
    pub gate: ComparisonNumbers,
    /// What the text of each name decided.
    pub text_ok: TextVerdicts,
    /// The comparisons the earlier ones left undecided.
    pub rest: ComparisonNumbers,
    /// The elements the row on the left found.
    pub stretched: Vec<Position>,
    /// The elements of the kinds a filter names.
    pub candidates: Candidates,
    /// The cells of the element being read.
    pub cells: [Value; 5],
    /// Whether a comparison reads a column the elements lack.
    pub missing: ColumnLack,
}

use ryo_values::domain::rows::types::RowIndex;
