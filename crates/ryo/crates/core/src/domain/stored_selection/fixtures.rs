//! Pure selector fixture shared by ownership and semantic tests.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::coverage::types::{Lacking, Partial};
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::OperatorContext;
use crate::domain::stored_indexes::service::{LIMIT, new};
use crate::domain::stored_selection::service::select;
use crate::domain::stored_selection::types::Selector;
use crate::domain::table_model::service::materialize;
use crate::domain::table_model::types::{
    ParameterBinding, Rows, SelectedRows, StoredSelection, Table,
};
use crate::domain::table_scratch::types::OperatorScratch;
use crate::domain::table_texts::types::SourceView;
use ryo_language::domain::relations::service::to_bits;
use ryo_language::domain::relations::types::{
    CellKind, ColumnsView, FactColumn, Integer, Relation, Value as Cell,
};
use ryo_language::domain::symbols::service::intern_derived;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_language::domain::symbols::types::Symbols;
use ryo_language::domain::terms::service::term_symbol;
use ryo_language::domain::terms::types::{DeclarationKind, NameText, Symbol};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Comparison, Input as Source, LiteralText, Operand, ParameterId, QueryLiteral,
    RepeatedRows, Test, WholeNumber,
};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::stored_facts::types::PartialCause;
use std::sync::Arc;

/// A predicate and a duplicate output map, with only its referenced bound value.
pub(crate) fn program(value: i64) -> Arc<Selector> {
    Arc::new(Selector {
        relation: Relation::Function,
        identity: Arc::new(crate::domain::table_model::types::SelectionIdentity {
            comparisons: vec![Comparison {
                test: Test::Equal,
                left: Operand::Column(ColumnIndex(0)),
                right: Operand::Parameter(ParameterId(0)),
                origin: None,
            }],
            bindings: vec![ParameterBinding::Number(WholeNumber(value))],
        }),
        columns: vec![ColumnIndex(1), ColumnIndex(0), ColumnIndex(1)],
        distinct: false,
    })
}
