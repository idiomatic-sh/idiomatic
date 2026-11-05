//! Finite ordinary Join driver over an engine-acquired immutable equality view.
use crate::domain::operating::{
    errors::OperatingError,
    types::{Emit, OperatorContext},
};
use crate::domain::operator::contract::{
    types::{PrimitiveSemantics, join_result},
    views::{BorrowedRows, IndexedRows, KeyProbe},
};

use crate::domain::table_model::service::row_in;
use ryo_operators::inbound::use_cases::operators::interface::{JoinDriver, JoinInterrupted, OperatorCall};
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::{ColumnIndex, JoinColumn};

pub(super) struct IndexedDriver<'a, 'f, 'c> {
    pub(super) index: IndexedRows<'f>,
    pub(super) context: &'a mut OperatorContext<'c>,
    pub(super) emit: &'a mut dyn Emit,
    pub(super) probe: Option<KeyProbe>,
    pub(super) current: Option<RowIndex>,
    pub(super) error: Option<OperatingError>,
}
impl JoinDriver for IndexedDriver<'_, '_, '_> {
    fn next(&mut self, key: &[Value]) -> Result<bool, JoinInterrupted> {
        if self.error.is_some() {
            return Err(JoinInterrupted);
        }
        let probe = self.probe.get_or_insert_with(|| self.index.probe(key));
        self.current = self.index.next_match(key, probe);
        #[cfg(feature = "diagnostic-row-progress")]
        if self.current.is_some() {
            (self.index.progress)(
                crate::domain::operator::contract::views::ProgressCounter::JoinCandidatePairs,
                1,
            );
        }
        if let (Some(row), BorrowedRows::Computed(table)) = (self.current, self.index.rows)
            && row_in(table, row).is_none()
        {
            self.error = Some(OperatingError::ShapeMismatch);
            return Err(JoinInterrupted);
        }
        Ok(self.current.is_some())
    }
    fn right_cell(&self, column: ColumnIndex) -> Option<Value> {
        if self.error.is_some() {
            return None;
        }
        self.current
            .and_then(|row| self.index.rows.cell(row, column))
    }
    fn emit(&mut self, row: &[Value]) -> Result<(), JoinInterrupted> {
        if self.error.is_some() {
            return Err(JoinInterrupted);
        }
        if let Err(error) = self.emit.row(self.context, row) {
            self.error = Some(error);
            return Err(JoinInterrupted);
        }
        Ok(())
    }
}
pub(super) fn indexed(
    semantics: &dyn PrimitiveSemantics,
    index: IndexedRows<'_>,
    left: &[Value],
    left_on: &[ColumnIndex],
    output: &[JoinColumn],
    key: &mut Vec<Value>,
    target: &mut Vec<Value>,
    context: &mut OperatorContext<'_>,
    emit: &mut dyn Emit,
) -> Result<(), OperatingError> {
    #[cfg(feature = "diagnostic-row-progress")]
    super::route_counts::row_file(left, context.paths);

    let mut driver = IndexedDriver {
        index,
        context,
        emit,
        probe: None,
        current: None,
        error: None,
    };
    let result = semantics.operators(OperatorCall::EqualityJoin {
        input: ryo_operators::inbound::use_cases::operators::interface::JoinInput::Values {
            left: ryo_operators::inbound::use_cases::operators::interface::JoinRow::Values(left),
            left_on,
            output,
            view: &[],
            driver: &mut driver,
            key,
            target,
        },
    });
    join_result(result, driver.error)
}

use ryo_values::domain::rows::types::RowIndex;
