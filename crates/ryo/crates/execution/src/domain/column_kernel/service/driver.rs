//! Pre-acquired matching rows and the already-proven total filter of a fused chain.
use crate::domain::column_kernel::types::Chain;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::views::{IndexedRows, KeyProbe};
use crate::domain::table_model::service::cell_at;
use crate::domain::table_model::types::Rows;
use ryo_language::domain::symbols::service::{hash_in, hash_of, text_of};
use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
use ryo_operators::domain::equality_join::types::JoinCells;
use ryo_operators::domain::equality_join::types::{Advance, JoinDriver};
use ryo_operators::domain::equality_join::errors::EqualityJoinError;
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::{JoinColumn, Side};
use ryo_values::domain::rows::service::push_row;
use ryo_values::domain::rows::types::{RowIndex, Table};
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
    fn next(&mut self, key: &[Value]) -> Result<Advance, EqualityJoinError> {
        if self.error.is_some() {
            return Err(EqualityJoinError::Interrupted);
        }
        let mut probe = self.probe.take().unwrap_or_else(|| self.index.probe(key));
        self.current = None;
        while let Some(right_row) = self.index.next_match(key, &mut probe) {
            #[cfg(feature = "diagnostic-row-progress")]
            self.index.progress.observe(
                crate::domain::operator::contract::views::ProgressCounter::JoinCandidatePairs,
                crate::domain::operator::contract::views::ProgressAmount(1),
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
                return Err(EqualityJoinError::Interrupted);
            };
            #[cfg(feature = "diagnostic-row-progress")]
            self.index.progress.observe(
                crate::domain::operator::contract::views::ProgressCounter::PredicateEvaluations,
                crate::domain::operator::contract::views::ProgressAmount(1),
            );
            if !symbol_holds(value, &self.chain.literal.0, self.symbols, self.source) {
                continue;
            }
            self.current = Some(right_row);
            break;
        }
        self.probe = Some(probe);
        Ok(Advance::from(self.current.is_some()))
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
    fn emit(&mut self, row: &[Value]) -> Result<(), EqualityJoinError> {
        if self.error.is_some() {
            return Err(EqualityJoinError::Interrupted);
        }
        #[cfg(feature = "diagnostic-row-progress")]
        self.index.progress.observe(
            crate::domain::operator::contract::views::ProgressCounter::KernelOutputRows,
            crate::domain::operator::contract::views::ProgressAmount(1),
        );
        push_row(self.target, row);
        self.emitted.set(self.emitted.get().saturating_add(1));
        Ok(())
    }
}

/// Resolve one physical cell, counting only actual explicit decodes.
pub(super) fn cell(
    left: &Rows<'_>,
    right: &Rows<'_>,
    left_row: RowIndex,
    right_row: RowIndex,
    source: JoinColumn,
    decoded: &std::cell::Cell<u64>,
) -> Option<Value> {
    let value = match source.side {
        Side::Left => cell_at(left, left_row, source.column),
        Side::Right => cell_at(right, right_row, source.column),
    }?;
    decoded.set(decoded.get().saturating_add(1));
    Some(value)
}

/// Match the ordinary literal-Text fast path, including exact hash/text checks.
pub(super) fn symbol_holds(
    value: Value,
    literal: &str,
    symbols: &Symbols,
    source: ByteSlice<'_>,
) -> bool {
    let Value::Symbol(symbol) = value else {
        return false;
    };
    let found = text_of(symbols, source, symbol);
    match hash_in(symbols, symbol) {
        Some(hash) => {
            hash == hash_of(ByteSlice(literal.as_bytes()))
                && found.is_some_and(|text| text.0 == literal)
        }
        // The ordinary evaluator falls through to its Datum text path when
        // a Symbol has no hash. Keep that behavior for malformed registrations.
        None => found.is_some_and(|text| text.0 == literal),
    }
}
