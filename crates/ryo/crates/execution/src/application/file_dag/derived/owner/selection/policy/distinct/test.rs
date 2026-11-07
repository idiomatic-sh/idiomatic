//! Distinct projections keep independent representatives and share only predicate matches.
use crate::application::file_dag::derived::owner::selection::fixtures::SelectionFixture;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::OperatorContext;
use crate::domain::stored_selection::fixtures::{expected_columns, program};
use crate::domain::table_model::service::materialize;
use crate::domain::table_model::types::Rows;
use crate::domain::table_scratch::types::OperatorScratch;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{CellKind, ColumnsView, FactColumn, Relation};
use ryo_values::domain::files::types::SourceView;
use ryo_values::domain::plan_parts::types::{ColumnIndex, WholeNumber};
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::rows::types::Table;
use std::sync::Arc;
type Failure = OperatingError;

/// Distinct projections keep independent first representatives while sharing only predicate matches.
#[test]
fn distinct_projections_preserve_rows_and_predicate_cache() -> Result<(), Failure> {
    let columns = [
        FactColumn(vec![7, 7, 7, 7, 7, 8]),
        // Unequal bit encodings of the same typed flag must deduplicate.
        FactColumn(vec![1, 2, 0, 7, 0, 1]),
        FactColumn(vec![10, 10, 20, 11, 20, 10]),
    ];
    let source = ColumnsView {
        kinds: &[CellKind::Number, CellKind::Flag, CellKind::Symbol],
        columns: &columns,
    };
    let mut first = (*program(7)).clone();
    first.columns = vec![ColumnIndex(2), ColumnIndex(1), ColumnIndex(2)];
    first.distinct = ryo_values::domain::plan_parts::types::RepeatedRows::Dropped;
    let first = Arc::new(first);
    let mut second = (*first).clone();
    second.columns = vec![ColumnIndex(1)];
    let second = Arc::new(second);
    let mut bag = (*first).clone();
    bag.distinct = ryo_values::domain::plan_parts::types::RepeatedRows::Kept;
    let bag = Arc::new(bag);
    let mut zero = (*first).clone();
    zero.columns.clear();
    let zero = Arc::new(zero);
    let mut absent = (*zero).clone();
    Arc::make_mut(&mut absent.identity).bindings = vec![ParameterBinding::Number(WholeNumber(999))];
    let absent = Arc::new(absent);
    let authored = vec![
        first.clone(),
        second.clone(),
        bag.clone(),
        zero.clone(),
        absent.clone(),
    ];
    let active = vec![first.clone(), second.clone()];
    let fixture = SelectionFixture::new(
        &[Relation::Function],
        &[],
        active.clone(),
        authored.clone(),
        source.kinds,
    )?;
    let mut symbols = Symbols::default();
    let mut context = OperatorContext {
        phase: crate::domain::operating::types::AfterTheFiles::Before,
        semantic: None,
        prepared_kinds: None,
        prepared_matchers: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(b""),
        paths: &[],
        arenas: &[],
        memo: &mut Vec::new(),
        scratch: &mut OperatorScratch::default(),
    };
    for (program, expected) in [(&first, vec![0, 2, 3]), (&second, vec![0, 2])] {
        let actual = fixture
            .select(program, source, &mut context)?
            .expect("compact distinct");
        assert_eq!(**actual.selection.rows, expected);
        let mut table = Table::default();
        materialize(
            &Rows::Selected(actual.rows.clone()),
            &mut table,
            &mut Vec::new(),
        );
        assert_eq!(table, expected_columns(source, &program.columns, &expected));
    }
    let bag_result = fixture
        .select(&bag, source, &mut context)?
        .expect("bag result");
    assert_eq!(**bag_result.selection.rows, [0, 1, 2, 3, 4]);
    let retained = fixture.snapshot(None);
    assert_eq!(retained.selections, 1);
    assert_eq!(
        retained.first_retained_rows_address,
        Some(Arc::as_ptr(&bag_result.selection.rows.0) as usize)
    );
    let selected = fixture
        .select(&zero, source, &mut context)?
        .expect("zero-width result");
    assert_eq!(**selected.selection.rows, [0]);
    let mut table = Table::default();
    materialize(
        &Rows::Selected(selected.rows.clone()),
        &mut table,
        &mut Vec::new(),
    );
    assert_eq!(table, expected_columns(source, &zero.columns, &[0]));
    assert_eq!((table.count.0, table.cells.len()), (1, 0));
    assert!(
        fixture
            .select(&absent, source, &mut context)?
            .expect("absent")
            .selection
            .rows
            .is_empty()
    );
    let empty_columns = [
        FactColumn(Vec::new()),
        FactColumn(Vec::new()),
        FactColumn(Vec::new()),
    ];
    let fresh = SelectionFixture::new(
        &[Relation::Function],
        &[],
        active.clone(),
        authored.clone(),
        source.kinds,
    )?;
    let empty = ColumnsView {
        columns: &empty_columns,
        ..source
    };
    assert!(
        fresh
            .select(&zero, empty, &mut context)?
            .expect("empty file")
            .selection
            .rows
            .is_empty()
    );
    let changed_columns = [
        FactColumn(vec![7]),
        FactColumn(vec![0]),
        FactColumn(vec![20]),
    ];
    let next_file =
        SelectionFixture::new(&[Relation::Function], &[], active, authored, source.kinds)?;
    assert_eq!(
        next_file.snapshot(None).retained_positions,
        0,
        "a new generation retains no prior positions"
    );
    let changed = ColumnsView {
        columns: &changed_columns,
        ..source
    };
    let actual = next_file
        .select(&first, changed, &mut context)?
        .expect("next file");
    assert_eq!(**actual.selection.rows, [0]);
    let mut table = Table::default();
    materialize(
        &Rows::Selected(actual.rows.clone()),
        &mut table,
        &mut Vec::new(),
    );
    assert_eq!(table, expected_columns(changed, &first.columns, &[0]));
    Ok(())
}
