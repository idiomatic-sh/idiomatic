//! The types of the `rows` port: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo_values::domain::plan_parts::types::Heading;

use ryo_values::domain::records::types::{Cell, CellText, RowTotal, Unread};

/// How many rows the answer has and how many of them were written: they differ when a limit cut the answer short.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Summary {
    /// Every row the answer has.
    pub total: RowTotal,
    /// The rows that were written.
    pub shown: RowTotal,
}

/// One cell of a row of the answer, lent while the port writes it: its value, typed, or that it has none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LentCell<'a> {
    /// A cell with a value.
    Valued(Cell<CellText<'a>>),
    /// A cell with no value.
    Empty,
}

impl<'a> From<Option<Cell<CellText<'a>>>> for LentCell<'a> {
    fn from(cell: Option<Cell<CellText<'a>>>) -> Self {
        cell.map_or(Self::Empty, Self::Valued)
    }
}

impl<'a> From<LentCell<'a>> for Option<Cell<CellText<'a>>> {
    fn from(cell: LentCell<'a>) -> Self {
        match cell {
            LentCell::Valued(value) => Some(value),
            LentCell::Empty => None,
        }
    }
}

/// One row of the answer, kept: its cells, typed; a cell with no value is `None`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Row(pub Box<[Option<Cell>]>);

/// Every row kept, for a caller that wants them all at once.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Collected {
    /// The headings of the columns.
    pub columns: Vec<Heading>,
    /// The rows, each cell typed.
    pub rows: Vec<Row>,
    /// The files nothing read.
    pub unread: Vec<Unread>,
    /// How many rows the answer has and how many were written.
    pub summary: Summary,
}

/// Where the collect adapter keeps every row, shared with the edge that takes them after the run: the edge makes it, hands a clone
/// to the factory, and takes what it holds once the run ends.
// ignore: idiomatic.arch/port/plain-port-types - the one way the rows of a run reach the edge that asked: the edge makes it, the collect adapter fills it while the run writes, and the edge takes the rows once the run ends; it is a shared place to put rows, not a value a contract compares
#[derive(Clone, Debug, Default)]
pub struct Collector(pub std::sync::Arc<std::sync::Mutex<Collected>>); // ignore: idiomatic.arch/port/plain-port-types - the lock is what the edge and the adapter share the rows through, as said above
