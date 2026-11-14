//! The types of the `semantic_rows` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo_derived::domain::row_cells::types::RowCells;
use ryo_execution_domain::domain::operator::contract::failure::types::FailureLatch;
use ryo_execution_domain::domain::table_model::types::RepresentationInput;
use ryo_operators::domain::table_texts::service::{name_text, symbol_text};
use ryo_operators::domain::table_texts::types::Texts;
use ryo_language::domain::symbols::{
    service::query_intern_derived,
    types::{ByteSlice, QuerySymbols},
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

pub(crate) struct AdmittedRows<'a, 'f> {
    pub(crate) rows: RepresentationInput<'f>,
    pub(crate) failure: &'a RefCell<FailureLatch>,
}

impl RowCells for AdmittedRows<'_, '_> {
    fn bulk(&self) -> Option<ryo_derived::domain::row_cells::types::BulkRows<'_>> {
        if ryo_execution_domain::domain::operator::contract::failure::service::failed(
            &*self.failure.borrow(),
        ) == ryo_execution_domain::domain::operator::contract::failure::types::LatchState::Failed
        {
            return None;
        }
        let Ok(rows) = ryo_execution_domain::domain::operator::contract::failure::service::run(
            &mut *self.failure.borrow_mut(),
            || self.rows.bulk(),
        ) else {
            return None;
        };
        match rows {
            RepresentationInput::Stored(source) => Some(
                ryo_derived::domain::row_cells::types::BulkRows::Stored(source),
            ),
            RepresentationInput::Computed(table) => Some(
                ryo_derived::domain::row_cells::types::BulkRows::Computed(table),
            ),
            RepresentationInput::Tree(_) | RepresentationInput::Compact(_) => None,
        }
    }
    fn kinds(&self) -> &[CellKind] {
        self.rows.kinds()
    }
    fn count(&self) -> RowCount {
        self.rows.count()
    }
    fn cell(&self, row: RowIndex, column: ColumnIndex) -> Option<Value> {
        if ryo_execution_domain::domain::operator::contract::failure::service::failed(
            &*self.failure.borrow(),
        ) == ryo_execution_domain::domain::operator::contract::failure::types::LatchState::Failed
        {
            return None;
        }
        self.rows.cell(row, column)
    }
    fn validate(&self) -> Result<(), ryo_derived::domain::fault::types::DerivedFault> {
        ryo_execution_domain::domain::operator::contract::failure::service::run(
            &mut *self.failure.borrow_mut(),
            || self.rows.validate(),
        )
        .map_err(|()| ryo_derived::domain::fault::types::DerivedFault::Interrupted)
    }
    fn read_row<'a>(&'a self, row: RowIndex, buffer: &'a mut Vec<Value>) -> Option<&'a [Value]> {
        if ryo_execution_domain::domain::operator::contract::failure::service::failed(
            &*self.failure.borrow(),
        ) == ryo_execution_domain::domain::operator::contract::failure::types::LatchState::Failed
        {
            return None;
        }
        self.rows.read(row, buffer)
    }
}

pub(crate) struct CanonicalTexts<'a, 's> {
    pub(crate) failure: &'a RefCell<FailureLatch>,
    pub(crate) symbols: &'a mut QuerySymbols<'s>,
    pub(crate) source: SourceView<'a>,
    pub(crate) paths: &'a [ryo_language::domain::files::types::FilePath],
    pub(crate) arenas: &'a [ryo_values::domain::stored_facts::types::ArenaBytes],
}

impl ryo_derived::domain::row_cells::types::ValueTexts for CanonicalTexts<'_, '_> {
    fn text(&self, value: Value) -> Option<CellText<'_>> {
        if ryo_execution_domain::domain::operator::contract::failure::service::failed(
            &*self.failure.borrow(),
        ) == ryo_execution_domain::domain::operator::contract::failure::types::LatchState::Failed
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
            Value::Node(_)
            | Value::File(_)
            | Value::Span(_)
            | Value::Number(_)
            | Value::Flag(_) => None,
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
        ryo_execution_domain::domain::operator::contract::failure::service::run(
            &mut *failure.borrow_mut(),
            || {
                Ok(Value::Symbol(query_intern_derived(
                    symbols,
                    ByteSlice(source.0),
                    NameText(text.0),
                )))
            },
        )
        .map_err(|()| ryo_derived::domain::fault::types::DerivedFault::Interrupted)
    }
}
