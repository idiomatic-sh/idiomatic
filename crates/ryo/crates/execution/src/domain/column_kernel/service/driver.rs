//! Pre-acquired matching rows and the already-proven total filter of a fused chain.
use crate::domain::column_kernel::types::{
    CellsDecoded, Chain, JoinedHits, RowsFiltered, SymbolVerdict,
};
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
use ryo_values::domain::plan_parts::types::{JoinColumn, LiteralText, Side};
use ryo_values::domain::rows::service::push_row;
use ryo_values::domain::rows::types::{RowIndex, Table};
pub(super) struct Left<'a, 'f> {
    pub(super) rows: &'a Rows<'f>,
    pub(super) row: RowIndex,
    pub(super) decoded: &'a std::cell::Cell<CellsDecoded>,
}
impl JoinCells for Left<'_, '_> {
    fn cell(&self, column: ryo_values::domain::plan_parts::types::ColumnIndex) -> Option<Value> {
        let value = cell_at(self.rows, self.row, column)?;
        self.decoded.set(CellsDecoded(self.decoded.get().0.saturating_add(1)));
        Some(value)
    }
}
pub(super) struct ChainDriver<'a, 'f> {
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
    pub(super) decoded: &'a std::cell::Cell<CellsDecoded>,
    pub(super) hits: &'a std::cell::Cell<JoinedHits>,
    pub(super) emitted: &'a std::cell::Cell<RowsFiltered>,
    pub(super) error: Option<OperatingError>,
}
impl JoinDriver for ChainDriver<'_, '_> {
    fn next(&mut self, key: &[Value]) -> Result<Advance, EqualityJoinError> {
        if self.error.is_some() {
            return Err(EqualityJoinError::Interrupted);
        }
        let mut probe = self.probe.take().unwrap_or_else(|| self.index.probe(key));
        self.current = None;
        while let Some(right_row) = self.index.next_match(key, &mut probe) {
            #[cfg(feature = "diagnostic-row-progress")]
            (self.index.progress)(
                crate::domain::operator::contract::views::ProgressCounter::JoinCandidatePairs,
                1,
            );
            self.hits.set(JoinedHits(self.hits.get().0.saturating_add(1)));
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
            (self.index.progress)(
                crate::domain::operator::contract::views::ProgressCounter::PredicateEvaluations,
                1,
            );
            if symbol_holds(value, &self.chain.literal, self.symbols, self.source)
                == SymbolVerdict::Differs
            {
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
        self.decoded.set(CellsDecoded(self.decoded.get().0.saturating_add(1)));
        Some(value)
    }
    fn emit(&mut self, row: &[Value]) -> Result<(), EqualityJoinError> {
        if self.error.is_some() {
            return Err(EqualityJoinError::Interrupted);
        }
        #[cfg(feature = "diagnostic-row-progress")]
        (self.index.progress)(
            crate::domain::operator::contract::views::ProgressCounter::KernelOutputRows,
            1,
        );
        push_row(self.target, row);
        self.emitted.set(RowsFiltered(self.emitted.get().0.saturating_add(1)));
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
    decoded: &std::cell::Cell<CellsDecoded>,
) -> Option<Value> {
    let value = match source.side {
        Side::Left => cell_at(left, left_row, source.column),
        Side::Right => cell_at(right, right_row, source.column),
    }?;
    decoded.set(CellsDecoded(decoded.get().0.saturating_add(1)));
    Some(value)
}

/// Match the ordinary literal-Text fast path, including exact hash/text checks.
pub(super) fn symbol_holds(
    value: Value,
    literal: &LiteralText,
    symbols: &Symbols,
    source: ByteSlice<'_>,
) -> SymbolVerdict {
    let literal: &str = &literal.0;
    let Value::Symbol(symbol) = value else {
        return SymbolVerdict::Differs;
    };
    let found = text_of(symbols, source, symbol);
    let holds = match hash_in(symbols, symbol) {
        Some(hash) => {
            hash == hash_of(ByteSlice(literal.as_bytes()))
                && found.is_some_and(|text| text.0 == literal)
        }
        // The ordinary evaluator falls through to its Datum text path when
        // a Symbol has no hash. Keep that behavior for malformed registrations.
        None => found.is_some_and(|text| text.0 == literal),
    };
    if holds {
        SymbolVerdict::Holds
    } else {
        SymbolVerdict::Differs
    }
}
