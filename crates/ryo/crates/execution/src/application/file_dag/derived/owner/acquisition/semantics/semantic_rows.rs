//! Borrowed adapters that present admitted rows and texts to the injected semantic port.
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::{failure::FailureLatch, };
use crate::domain::table_model::types::RepresentationInput as BorrowedRows;
use crate::domain::table_texts::service::{datum, name_text, symbol_text};
use crate::domain::table_texts::types::{Datum, Texts};
use ryo_derived::domain::contract::types::{self as semantic, RowCells};
use ryo_language::domain::symbols::{
    service::query_intern_derived,
    types::{ByteSlice, QuerySymbols, Symbols},
};
use ryo_relations::domain::{
    relations::types::{CellKind, Value},
    terms::types::NameText,
};
use ryo_values::domain::{
    files::types::SourceView, plan_parts::types::ColumnIndex, rows::types::RowIndex,
};
use std::cell::RefCell;

pub(super) struct AdmittedRows<'a, 'f> {
    pub(super) rows: BorrowedRows<'f>,
    pub(super) failure: &'a RefCell<FailureLatch>,
}
impl RowCells for AdmittedRows<'_, '_> {
    fn bulk(&self) -> Option<semantic::BulkRows<'_>> {
        if self.failure.borrow().failed()
            == crate::domain::operator::contract::failure::LatchState::Failed
        {
            return None;
        }
        let Ok(rows) = self.failure.borrow_mut().run(|| self.rows.bulk()) else {
            return None;
        };
        match rows {
            BorrowedRows::Stored(source) => Some(semantic::BulkRows::Stored(source)),
            BorrowedRows::Selected {
                source,
                positions,
                columns,
                ..
            } => Some(semantic::BulkRows::Selected {
                source,
                positions: positions.0,
                columns,
            }),
            BorrowedRows::Computed(table) => Some(semantic::BulkRows::Computed(table)),
            BorrowedRows::Tree(_) | BorrowedRows::Compact(_) | BorrowedRows::Positioned(_) => None,
        }
    }
    fn kinds(&self) -> &[CellKind] {
        self.rows.kinds()
    }
    fn count(&self) -> usize {
        self.rows.count().0
    }
    fn cell(&self, row: usize, column: ColumnIndex) -> Option<Value> {
        if self.failure.borrow().failed()
            == crate::domain::operator::contract::failure::LatchState::Failed
        {
            return None;
        }
        self.rows.cell(RowIndex(row), column)
    }
    fn validate(&self) -> Result<(), semantic::DerivedFault> {
        self.failure
            .borrow_mut()
            .run(|| self.rows.validate())
            .map_err(|()| semantic::DerivedFault::Interrupted)
    }
    fn read_row<'a>(&'a self, row: usize, buffer: &'a mut Vec<Value>) -> Option<&'a [Value]> {
        if self.failure.borrow().failed()
            == crate::domain::operator::contract::failure::LatchState::Failed
        {
            return None;
        }
        self.rows.read(RowIndex(row), buffer)
    }
}
pub(super) struct Predicate<'a, 'f> {
    pub(super) rows: BorrowedRows<'f>,
    pub(super) program: &'a mut dyn crate::domain::operator::contract::views::PredicateProgram,
    pub(super) symbols: &'a Symbols,
    pub(super) needed: &'a crate::domain::table_cells::types::ColumnPlaces,
    pub(super) row: &'a mut Vec<Value>,
    pub(super) failure: &'a RefCell<FailureLatch>,
}
impl semantic::PredicateRows for Predicate<'_, '_> {
    fn count(&self) -> usize {
        self.rows.count().0
    }
    fn holds(&mut self, at: usize) -> Result<bool, semantic::DerivedFault> {
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
                let row = if let BorrowedRows::Stored(source) = rows {
                    row.resize(
                        source.kinds.len(),
                        Value::Flag(ryo_relations::domain::relations::types::Flag::Lowered),
                    );
                    if crate::domain::table_cells::service::read_cells(
                        source,
                        RowIndex(at),
                        row,
                        needed,
                    ) != crate::domain::table_cells::types::CellsRead::Read
                    {
                        return Err(OperatingError::ShapeMismatch);
                    }
                    row.as_slice()
                } else {
                    rows.read(RowIndex(at), row)
                        .ok_or(OperatingError::ShapeMismatch)?
                };
                Ok(program.holds(row, symbols)?
                    == crate::domain::table_model::types::Verdict::Holds)
            })
            .map_err(|()| semantic::DerivedFault::Interrupted)
    }
}
impl semantic::ValueTexts for Texts<'_> {
    fn text(&self, value: Value) -> Option<&str> {
        match datum(value, self) {
            Datum::Text(text) => Some(text.0),
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
impl semantic::ValueTexts for CanonicalTexts<'_, '_> {
    fn text(&self, value: Value) -> Option<&str> {
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
            Value::Symbol(symbol) => symbol_text(&texts, symbol).map(|text| text.0),
            Value::Name(name) => name_text(&texts, name).map(|text| text.0),
            _ => None,
        }
    }
}
impl semantic::CanonicalTexts for CanonicalTexts<'_, '_> {
    fn intern(&mut self, text: &str) -> Result<Value, semantic::DerivedFault> {
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
                    NameText(text),
                )))
            })
            .map_err(|()| semantic::DerivedFault::Interrupted)
    }
}
