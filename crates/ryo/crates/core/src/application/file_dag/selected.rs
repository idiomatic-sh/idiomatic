//! Generation-local selected-row index and direct projected-cell emission.
//! Only the application retains selection metadata; primitives receive borrowed typed rows.

use crate::domain::operating::{
    errors::OperatingError,
    types::{Emit, OperatorContext},
};
use crate::domain::operator::contract::views::{BorrowedRows, IndexedRows};
use crate::domain::table_model::{
    service::{index_selected, joined_selected, key_of, matching_selected},
    types::{KeyIndex, On, SelectedRows},
};
use ryo_language::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::{ColumnIndex, JoinColumn};

/// One complete selected population and an index over only its declared key cells.
/// The source remains borrowed from this physical file; metadata is an Arc clone.
pub(crate) struct SelectedInput<'f> {
    original: SelectedRows<'f>,
    columns: Vec<ColumnIndex>,
    index: KeyIndex,
}

impl<'f> SelectedInput<'f> {
    /// Reject malformed source/selection metadata before indexing any row.
    pub(crate) fn prepare(
        original: &SelectedRows<'f>,
        columns: &[ColumnIndex],
    ) -> Result<Self, OperatingError> {
        let original = original.clone();
        let mut result = Self {
            original,
            columns: columns.to_vec(),
            index: KeyIndex::default(),
        };
        result.view().rows.validate()?;
        index_selected(&result.original, &result.columns, &mut result.index)?;
        Ok(result)
    }

    /// Borrow only the physical columns and compact positions, never an owned selection handle.
    pub(crate) fn view(&self) -> IndexedRows<'_> {
        IndexedRows {
            rows: BorrowedRows::Selected {
                source: self.original.source,
                positions: &self.original.selection.rows,
                columns: &self.original.selection.columns,
                kinds: &self.original.selection.kinds,
            },
            columns: &self.columns,
            index: &self.index,
        }
    }

    /// Join one left row with matching compact positions in their original bag order.
    /// Only output cells are decoded; no full RHS row or table is materialized.
    pub(crate) fn emit_join(
        &self,
        left_on: &[ColumnIndex],
        output: &[JoinColumn],
        left: &[Value],
        key: &mut Vec<Value>,
        answer: &mut Vec<Value>,
        context: &mut OperatorContext<'_>,
        emit: &mut dyn Emit,
    ) -> Result<(), OperatingError> {
        key_of(left, On::Columns(left_on), key)?;
        for row in matching_selected(&self.original, &self.columns, &self.index, key) {
            joined_selected(&self.original, row, left, output, answer)?;
            emit.row(context, answer)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
