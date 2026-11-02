//! Pure private admission/storage algorithms remain beside their domain owner.
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

use super::fixtures::program;
type Failure = OperatingError;
/// Entries append into spare owned capacity instead of moving every retained entry.
#[test]
fn retained_selections_keep_spare_vector_capacity() -> Result<(), Failure> {
    let columns: Vec<_> = (0_i64..3)
        .map(|column| {
            FactColumn(
                (0_i64..4)
                    .map(|row| to_bits(Cell::Number(Integer(row + column))).0)
                    .collect(),
            )
        })
        .collect();
    let source = ColumnsView {
        kinds: &[CellKind::Number; 3],
        columns: &columns,
    };
    let (first, second) = (program(2), program(3));
    let mut cache = new(&[Relation::Function], &[], LIMIT).expect("generation");
    cache.selectors = vec![
        Arc::clone(&first),
        Arc::clone(&first),
        Arc::clone(&second),
        Arc::clone(&second),
    ];
    cache
        .selections
        .try_reserve_exact(8)
        .expect("test spare capacity");
    let storage = cache.selections.as_ptr();
    let before = cache.bytes;
    let mut symbols = Symbols::default();
    let mut memo = Vec::new();
    let mut scratch = OperatorScratch::default();
    let mut context = OperatorContext {
        prepared_kinds: None,
        prepared_matchers: None,
        stored_indexes: Some(&mut cache),
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(b""),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    let selected = select(&first, source, &mut context)?.expect("first result");
    let selected_next = select(&second, source, &mut context)?.expect("second result");
    assert_ne!(selected.selection.rows, selected_next.selection.rows);
    let retained = context.stored_indexes.as_ref().expect("cache restored");
    assert_eq!(retained.selections.len(), 2);
    assert_eq!(retained.selections.as_ptr(), storage);
    assert_eq!(
        retained.bytes,
        before + crate::domain::stored_selection::service::RETAINED_SELECTION_RESERVATION
    );
    Ok(())
}

/// A result larger than the fixed selection allowance stays local to its call.
#[test]
fn oversized_selection_refuses_retention_without_charging_the_cache() {
    let program = program(2);
    let mut cache = new(&[Relation::Function], &[], LIMIT).expect("generation");
    cache.selectors = vec![Arc::clone(&program), Arc::clone(&program)];
    let before = cache.bytes;
    let rows = Arc::new(Vec::with_capacity(
        crate::domain::stored_selection::service::MAX_RETAINED_POSITION_CAPACITY + 1,
    ));
    let columns = [
        FactColumn::default(),
        FactColumn::default(),
        FactColumn::default(),
    ];
    let source = ColumnsView {
        kinds: &[CellKind::Number; 3],
        columns: &columns,
    };
    let _admission =
        crate::domain::stored_selection::service::admit(&mut cache, &program, source, 0, &rows);
    assert!(cache.selections.is_empty());
    assert_eq!(cache.selection_counts.refusals, 1);
    assert_eq!(cache.bytes, before);
}

#[test]
fn distinct_representatives_handle_collisions_and_invalid_cells() {
    let columns = [
        FactColumn(vec![7, 7, 7, 7, 7, 8]),
        FactColumn(vec![1, 2, 0, 7, 0, 1]),
        FactColumn(vec![10, 10, 20, 11, 20, 10]),
    ];
    let source = ColumnsView {
        kinds: &[CellKind::Number, CellKind::Flag, CellKind::Symbol],
        columns: &columns,
    };
    let constant_hash = |_: ColumnsView<'_>, _: &[ColumnIndex], _: u32| Some(0);
    assert_eq!(
        crate::domain::stored_selection::service::representatives(
            source,
            &[ColumnIndex(2), ColumnIndex(1), ColumnIndex(2)],
            &[0, 1, 2, 3, 4],
            constant_hash
        ),
        Some(vec![0, 2, 3]),
    );
    assert_eq!(
        crate::domain::stored_selection::service::representatives(
            source,
            &[ColumnIndex(2), ColumnIndex(1), ColumnIndex(2)],
            &[0, 99],
            constant_hash
        ),
        None,
    );
    assert_eq!(
        crate::domain::stored_selection::service::representatives(
            source,
            &[ColumnIndex(99)],
            &[0],
            constant_hash
        ),
        None,
    );
}
