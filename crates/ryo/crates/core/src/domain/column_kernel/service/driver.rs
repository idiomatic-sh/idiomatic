//! Pre-acquired matching rows and the already-proven total filter of a fused chain.
use super::*;
use crate::domain::operator::contract::views::KeyProbe;
use crate::domain::table_model::service::push_row;
use ryo_operators::domain::contract::types::{JoinCells, JoinDriver, JoinInterrupted};
pub(super) struct Left<'a, 'f> {
    pub(super) rows: &'a Rows<'f>,
    pub(super) row: RowIndex,
    pub(super) decoded: &'a std::cell::Cell<u64>,
}
impl JoinCells for Left<'_, '_> {
    fn cell(&self, column: ryo_values::domain::plan_parts::types::ColumnIndex) -> Option<Value> {
        let value = cell_at(self.rows, self.row, column)?;
        self.decoded.set(self.decoded.get().saturating_add(1));
        Some(value)
    }
}
pub(super) struct Driver<'a, 'f> {
    pub(super) chain: &'a Chain,
    pub(super) left: &'a Rows<'f>,
    pub(super) left_row: RowIndex,
    pub(super) right: &'a Rows<'f>,
    pub(super) index: IndexedRows<'a>,
    pub(super) probe: Option<KeyProbe>,
    pub(super) current: Option<RowIndex>,
    pub(super) symbols: &'a Symbols,
    pub(super) source: ByteSlice<'a>,
    pub(super) target: &'a mut Table,
    pub(super) decoded: &'a std::cell::Cell<u64>,
    pub(super) hits: &'a std::cell::Cell<u64>,
    pub(super) emitted: &'a std::cell::Cell<u64>,
    pub(super) error: Option<OperatingError>,
}
impl JoinDriver for Driver<'_, '_> {
    fn next(&mut self, key: &[Value]) -> Result<bool, JoinInterrupted> {
        if self.error.is_some() {
            return Err(JoinInterrupted);
        }
        let mut probe = self.probe.take().unwrap_or_else(|| self.index.probe(key));
        self.current = None;
        while let Some(right_row) = self.index.next_match(key, &mut probe) {
            #[cfg(feature = "diagnostic-row-progress")]
            (self.index.progress)(
                crate::domain::operator::contract::views::ProgressCounter::JoinCandidatePairs,
                1,
            );
            self.hits.set(self.hits.get().saturating_add(1));
            let Some(value) = cell(
                self.left,
                self.right,
                self.left_row,
                right_row,
                self.chain.filter,
                self.decoded,
            ) else {
                self.error = Some(OperatingError::ShapeMismatch);
                return Err(JoinInterrupted);
            };
            #[cfg(feature = "diagnostic-row-progress")]
            (self.index.progress)(
                crate::domain::operator::contract::views::ProgressCounter::PredicateEvaluations,
                1,
            );
            if !symbol_holds(value, &self.chain.literal.0, self.symbols, self.source) {
                continue;
            }
            self.current = Some(right_row);
            break;
        }
        self.probe = Some(probe);
        Ok(self.current.is_some())
    }
    fn right_cell(
        &self,
        column: ryo_values::domain::plan_parts::types::ColumnIndex,
    ) -> Option<Value> {
        if self.error.is_some() {
            return None;
        }
        let value = self.index.rows.cell(self.current?, column)?;
        self.decoded.set(self.decoded.get().saturating_add(1));
        Some(value)
    }
    fn emit(&mut self, row: &[Value]) -> Result<(), JoinInterrupted> {
        if self.error.is_some() {
            return Err(JoinInterrupted);
        }
        #[cfg(feature = "diagnostic-row-progress")]
        (self.index.progress)(
            crate::domain::operator::contract::views::ProgressCounter::KernelOutputRows,
            1,
        );
        push_row(self.target, row);
        self.emitted.set(self.emitted.get().saturating_add(1));
        Ok(())
    }
}
