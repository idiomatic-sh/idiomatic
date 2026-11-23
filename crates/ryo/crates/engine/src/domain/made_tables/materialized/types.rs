//! Storage retained by one worker and the answers borrowed by its consumers.
use super::constants::CLASSES;
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::CellKind;
use ryo_relations::domain::relations::types::Count;
use ryo_values::domain::descriptions::types::ColumnName;
use std::rc::Rc;

/// Columns in projection order and the number of rows, independent of column count.
/// The cell carrier is fixed for the whole table. File adapters can lend their native
/// packed words directly; callers using semantic cell words use the same services.
#[derive(Debug, Clone)]
pub struct ResultColumns<Word = CellWord, Name = ColumnName> {
    /// Plan-owned column names in projection order.
    pub names: Rc<Vec<Name>>,
    /// Plan-owned logical kinds in the same order.
    pub tys: Rc<Vec<CellKind>>,
    /// Owned homogeneous buffers. A published table is shared by its consumers.
    pub cols: Vec<Vec<Word>>,
    /// Logical row count, including rows with no columns.
    pub rows: Count,
}

/// One worker owns all scratch and reuse budgets. No borrow lives across a new file.
#[derive(Debug)]
pub struct TableBuffers<Word = CellWord, Name = ColumnName> {
    /// Cleared columns, ordered by capacity class.
    pub columns: [Vec<Vec<Word>>; CLASSES],
    /// Total retained column capacity in cells.
    pub capacity: Count,
    /// Cleared vectors of columns.
    pub headers: Vec<Vec<Vec<Word>>>,
    /// Uniquely held shared answers.
    pub shared: Vec<Rc<ResultColumns<Word, Name>>>,
}

/// Cleared, uniquely held selections retained by a worker, across file lifetimes.
#[derive(Debug, Default)]
pub struct SelectionBuffers<Row> {
    /// Shared empty selection.
    pub empty: Rc<Vec<Row>>,
    /// Uniquely owned selections waiting to be filled.
    pub retained: Vec<Rc<Vec<Row>>>,
    /// Retained capacity in rows.
    pub capacity: Count,
}

impl<Word, Name> Default for TableBuffers<Word, Name> {
    fn default() -> Self {
        Self {
            columns: Default::default(),
            capacity: Count(0),
            headers: Vec::new(),
            shared: Vec::new(),
        }
    }
}

impl<Word, Name> Default for ResultColumns<Word, Name> {
    fn default() -> Self {
        Self {
            names: Rc::default(),
            tys: Rc::default(),
            cols: Vec::new(),
            rows: Count(0),
        }
    }
}
