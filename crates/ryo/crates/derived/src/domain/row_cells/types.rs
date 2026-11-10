//! The types of the `rows` concept: plain data, public fields and derives, with no constructor and no inherent impl.
use crate::domain::fault::types::DerivedFault;
use ryo_relations::domain::relations::types::{CellKind, ColumnsView, Value};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::row_texts::types::CellText;
use ryo_values::domain::rows::types::{RowCount, RowIndex, Table};

/// A finite already-admitted row view. Reading a cell cannot schedule upstream work.
#[derive(Clone, Copy)]
pub enum BulkRows<'a> {
    /// Rows stored in physical columns.
    Stored(ColumnsView<'a>),
    /// Rows computed into a table.
    Computed(&'a Table),
}

/// Finite rows the derived builders read cell by cell.
pub trait RowCells {
    /// Decide once per build whether physical bulk decoding is available.
    fn bulk(&self) -> Option<BulkRows<'_>>;

    /// Reads one whole row into the buffer.
    fn read_row<'a>(&'a self, row: RowIndex, buffer: &'a mut Vec<Value>) -> Option<&'a [Value]> {
        if row.0 >= self.count().0 {
            return None;
        }
        buffer.clear();
        for column in 0..self.kinds().len() {
            let Ok(index) = u32::try_from(column) else {
                return None;
            };
            buffer.push(self.cell(row, ColumnIndex(index))?);
        }
        Some(buffer.as_slice())
    }
    /// Checks that the rows are well formed.
    ///
    /// # Errors
    ///
    /// [`DerivedFault::ShapeMismatch`] when the cells are not one per column of every row.
    fn validate(&self) -> Result<(), DerivedFault>;
    /// The kind of each column.
    fn kinds(&self) -> &[CellKind];
    /// The number of rows.
    fn count(&self) -> RowCount;
    /// One cell, absent when the row or column does not exist.
    fn cell(&self, row: RowIndex, column: ColumnIndex) -> Option<Value>;
}

/// Finite textual decoding only; cannot schedule or retain a population.
pub trait ValueTexts {
    /// The text a symbol or name cell stands for.
    fn text(&self, value: Value) -> Option<CellText<'_>>;
}

/// Source text access and query-symbol storage are finite data services, not acquisition policy.
pub trait CanonicalTexts: ValueTexts {
    /// Interns a text and answers the cell standing for it.
    ///
    /// # Errors
    ///
    /// A [`DerivedFault`] when the symbol storage cannot hold the text.
    fn intern(&mut self, text: CellText<'_>) -> Result<Value, DerivedFault>;
}

impl RowCells for Table {
    fn bulk(&self) -> Option<BulkRows<'_>> {
        Some(BulkRows::Computed(self))
    }
    fn validate(&self) -> Result<(), DerivedFault> {
        if self.kinds.len().checked_mul(self.count.0) != Some(self.cells.len()) {
            return Err(DerivedFault::ShapeMismatch);
        }
        Ok(())
    }
    fn kinds(&self) -> &[CellKind] {
        &self.kinds
    }
    fn count(&self) -> RowCount {
        self.count
    }
    fn cell(&self, row: RowIndex, column: ColumnIndex) -> Option<Value> {
        if row.0 >= self.count.0 {
            return None;
        }
        let column = usize::from(column);
        if column >= self.kinds.len() {
            return None;
        }
        self.cells
            .get(row.0.checked_mul(self.kinds.len())?.checked_add(column)?)
            .copied()
    }
}
