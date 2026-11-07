//! Borrowed adapters that present admitted rows and texts to the injected semantic port.
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::{failure::FailureLatch, };
use crate::domain::table_model::types::RepresentationInput;
use crate::domain::table_texts::service::{datum, name_text, symbol_text};
use crate::domain::table_texts::types::{Datum, Texts};
use ryo_derived::domain::row_cells::types::RowCells;
use ryo_language::domain::symbols::{
    service::query_intern_derived,
    types::{ByteSlice, QuerySymbols, Symbols},
};
use ryo_relations::domain::{
    relations::types::{CellKind, Value},
    terms::types::NameText,
};
use ryo_values::domain::{
    files::types::SourceView,
    plan_parts::types::ColumnIndex,
    row_texts::types::CellText,
    rows::types::{RowCount, RowIndex},
};
use std::cell::RefCell;

pub(super) struct AdmittedRows<'a, 'f> {
    pub(super) rows: RepresentationInput<'f>,
    pub(super) failure: &'a RefCell<FailureLatch>,
}
impl RowCells for AdmittedRows<'_, '_> {
    fn bulk(&self) -> Option<ryo_derived::domain::row_cells::types::BulkRows<'_>> {
        if self.failure.borrow().failed()
            == crate::domain::operator::contract::failure::LatchState::Failed
        {
            return None;
        }
        let Ok(rows) = self.failure.borrow_mut().run(|| self.rows.bulk()) else {
            return None;
        };
        match rows {
            RepresentationInput::Stored(source) => Some(ryo_derived::domain::row_cells::types::BulkRows::Stored(source)),
            RepresentationInput::Selected {
                source,
                positions,
                columns,
                ..
            } => Some(ryo_derived::domain::row_cells::types::BulkRows::Selected {
                source,
                positions: ryo_derived::domain::positions::types::RowSlots(positions.0),
                columns,
            }),
            RepresentationInput::Computed(table) => Some(ryo_derived::domain::row_cells::types::BulkRows::Computed(table)),
            RepresentationInput::Tree(_) | RepresentationInput::Compact(_) | RepresentationInput::Positioned(_) => None,
        }
    }
    fn kinds(&self) -> &[CellKind] {
        self.rows.kinds()
    }
    fn count(&self) -> RowCount {
        self.rows.count()
    }
    fn cell(&self, row: RowIndex, column: ColumnIndex) -> Option<Value> {
        if self.failure.borrow().failed()
            == crate::domain::operator::contract::failure::LatchState::Failed
        {
            return None;
        }
        self.rows.cell(row, column)
    }
    fn validate(&self) -> Result<(), ryo_derived::domain::fault::types::DerivedFault> {
        self.failure
            .borrow_mut()
            .run(|| self.rows.validate())
            .map_err(|()| ryo_derived::domain::fault::types::DerivedFault::Interrupted)
    }
    fn read_row<'a>(&'a self, row: RowIndex, buffer: &'a mut Vec<Value>) -> Option<&'a [Value]> {
        if self.failure.borrow().failed()
            == crate::domain::operator::contract::failure::LatchState::Failed
        {
            return None;
        }
        self.rows.read(row, buffer)
    }
}
pub(super) struct Predicate<'a, 'f> {
    pub(super) rows: RepresentationInput<'f>,
    pub(super) program: &'a mut dyn crate::domain::operator::contract::views::PredicateProgram,
    pub(super) symbols: &'a Symbols,
    pub(super) needed: &'a crate::domain::table_cells::types::ColumnPlaces,
    pub(super) row: &'a mut Vec<Value>,
    pub(super) failure: &'a RefCell<FailureLatch>,
}
impl ryo_derived::domain::row_cells::types::PredicateRows for Predicate<'_, '_> {
    fn count(&self) -> RowCount {
        self.rows.count()
    }
    fn holds(
        &mut self,
        at: RowIndex,
    ) -> Result<
        ryo_derived::domain::row_cells::types::PredicateVerdict,
        ryo_derived::domain::fault::types::DerivedFault,
    > {
        let Self {
            rows,
            program,
            symbols,
            needed,
            row,
            failure,
        } = self;
        failure
            .borrow_mut()
            .run(|| {
                let row = if let RepresentationInput::Stored(source) = rows {
                    row.resize(
                        source.kinds.len(),
                        Value::Flag(ryo_relations::domain::relations::types::Flag::Lowered),
                    );
                    if crate::domain::table_cells::service::read_cells(
                        source,
                        at,
                        row,
                        needed,
                    ) != crate::domain::table_cells::types::CellsRead::Read
                    {
                        return Err(OperatingError::ShapeMismatch);
                    }
                    row.as_slice()
                } else {
                    rows.read(at, row)
                        .ok_or(OperatingError::ShapeMismatch)?
                };
                Ok(
                    if program.holds(row, symbols)?
                        == crate::domain::table_model::types::Verdict::Holds
                    {
                        ryo_derived::domain::row_cells::types::PredicateVerdict::Holds
                    } else {
                        ryo_derived::domain::row_cells::types::PredicateVerdict::Fails
                    },
                )
            })
            .map_err(|()| ryo_derived::domain::fault::types::DerivedFault::Interrupted)
    }
}
impl ryo_derived::domain::row_cells::types::ValueTexts for Texts<'_> {
    fn text(&self, value: Value) -> Option<CellText<'_>> {
        match datum(value, self) {
            Datum::Text(text) => Some(CellText(text.0)),
            _ => None,
        }
    }
}
pub(super) struct CanonicalTexts<'a, 's> {
    pub(super) failure: &'a RefCell<FailureLatch>,
    pub(super) symbols: &'a mut QuerySymbols<'s>,
    pub(super) source: SourceView<'a>,
    pub(super) paths: &'a [ryo_language::domain::files::types::FilePath],
    pub(super) arenas: &'a [ryo_values::domain::stored_facts::types::ArenaBytes],
}
impl ryo_derived::domain::row_cells::types::ValueTexts for CanonicalTexts<'_, '_> {
    fn text(&self, value: Value) -> Option<CellText<'_>> {
        if self.failure.borrow().failed()
            == crate::domain::operator::contract::failure::LatchState::Failed
        {
            return None;
        }
        let texts = Texts {
            symbols: self.symbols.as_ref(),
            source: self.source,
            paths: self.paths,
            arenas: self.arenas,
        };
        match value {
            Value::Symbol(symbol) => symbol_text(&texts, symbol).map(|text| CellText(text.0)),
            Value::Name(name) => name_text(&texts, name).map(|text| CellText(text.0)),
            _ => None,
        }
    }
}
impl ryo_derived::domain::row_cells::types::CanonicalTexts for CanonicalTexts<'_, '_> {
    fn intern(
        &mut self,
        text: CellText<'_>,
    ) -> Result<Value, ryo_derived::domain::fault::types::DerivedFault> {
        let Self {
            failure,
            symbols,
            source,
            ..
        } = self;
        failure
            .borrow_mut()
            .run(|| {
                Ok(Value::Symbol(query_intern_derived(
                    symbols,
                    ByteSlice(source.0),
                    NameText(text.0),
                )))
            })
            .map_err(|()| ryo_derived::domain::fault::types::DerivedFault::Interrupted)
    }
}
