//! Admission and once-charged cache policy are exercised under their application owner.
use super::select;
use crate::application::file_dag::derived::indexes::{LIMIT, new};
use crate::domain::{
    operating::{errors::OperatingError, types::OperatorContext},
    stored_selection::fixtures::program,
    table_scratch::types::OperatorScratch,
    table_texts::types::SourceView,
};
use ryo_language::domain::{
    relations::{
        service::to_bits,
        types::{CellKind, ColumnsView, FactColumn, Integer, Relation, Value as Cell},
    },
    symbols::types::Symbols,
};
use std::sync::Arc;
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
    {
        let prepared_views =
            super::prepare_views_with_schemas(cache.selectors.iter().cloned(), &|_| source.kinds)?;
        crate::application::file_dag::derived::selection::install_views(&mut cache, prepared_views);
    }
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
    let descriptor = context
        .stored_indexes
        .as_ref()
        .unwrap()
        .selector_descriptor
        .unwrap();
    let expected = Arc::as_ptr(
        &context
            .stored_indexes
            .as_ref()
            .unwrap()
            .selection_views
            .as_ref()
            .unwrap()
            .by_selector[&(Arc::as_ptr(&first) as usize)],
    ) as usize;
    assert_eq!(
        descriptor, expected,
        "actual selector acquisition shares request descriptor allocation"
    );
    let predicate_cells = context
        .stored_indexes
        .as_ref()
        .unwrap()
        .selection_counts
        .predicate_cells;
    let repeated = select(&first, source, &mut context)?.expect("shared repeated result");
    assert!(Arc::ptr_eq(
        &selected.selection.rows,
        &repeated.selection.rows
    ));
    assert_eq!(
        context.stored_indexes.as_ref().unwrap().selector_descriptor,
        Some(descriptor)
    );
    assert_eq!(
        context
            .stored_indexes
            .as_ref()
            .unwrap()
            .selection_counts
            .predicate_cells,
        predicate_cells
    );
    let selected_next = select(&second, source, &mut context)?.expect("second result");
    assert_ne!(selected.selection.rows, selected_next.selection.rows);
    let retained = context.stored_indexes.as_ref().expect("cache restored");
    assert_eq!(retained.selections.len(), 2);
    assert_eq!(
        retained.retained_positions,
        retained
            .selections
            .iter()
            .map(|entry| entry.rows.capacity())
            .sum::<usize>()
    );
    assert_eq!(retained.selections.as_ptr(), storage);
    assert_eq!(
        retained.bytes,
        before + super::RETAINED_SELECTION_RESERVATION
    );
    Ok(())
}

/// A result larger than the fixed selection allowance stays local to its call.
#[test]
fn oversized_selection_refuses_retention_without_charging_the_cache() -> Result<(), Failure> {
    let program = program(2);
    let mut cache = new(&[Relation::Function], &[], LIMIT).expect("generation");
    cache.selectors = vec![Arc::clone(&program), Arc::clone(&program)];
    let before = cache.bytes;
    let rows = Arc::new(Vec::with_capacity(
        super::MAX_RETAINED_POSITION_CAPACITY + 1,
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
    let prepared = super::prepare_views_with_schemas([Arc::clone(&program)], &|_| source.kinds)?;
    let descriptor = Arc::clone(&prepared.by_selector[&(Arc::as_ptr(&program) as usize)]);
    super::install_views(&mut cache, prepared);
    let _admission = super::admit(&mut cache, &program, &descriptor, source, 0, &rows)?;
    assert!(cache.selections.is_empty());
    assert_eq!(cache.retained_positions, 0);
    assert_eq!(cache.selection_counts.refusals, 1);
    assert_eq!(cache.bytes, before);
    Ok(())
}

#[test]
fn selecting_never_prepares_an_unknown_selector_identity() -> Result<(), Failure> {
    let columns = [
        FactColumn(vec![1, 2]),
        FactColumn(vec![3, 4]),
        FactColumn(vec![5, 6]),
    ];
    let source = ColumnsView {
        kinds: &[CellKind::Number; 3],
        columns: &columns,
    };
    let known = program(2);
    let unknown = Arc::new((*known).clone());
    let mut cache = new(&[Relation::Function], &[], LIMIT).expect("generation");
    cache.selectors = vec![Arc::clone(&known)];
    let views =
        super::prepare_views_with_schemas(cache.selectors.iter().cloned(), &|_| source.kinds)?;
    {
        let prepared_views = Arc::clone(&views);
        crate::application::file_dag::derived::selection::install_views(&mut cache, prepared_views);
    }
    let (mut symbols, mut memo, mut scratch) =
        (Symbols::default(), Vec::new(), OperatorScratch::default());
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
    assert!(select(&unknown, source, &mut context)?.is_none());
    assert_eq!(
        context
            .stored_indexes
            .as_ref()
            .unwrap()
            .selection_counts
            .builds,
        0
    );
    assert!(Arc::ptr_eq(
        context
            .stored_indexes
            .as_ref()
            .unwrap()
            .selection_views
            .as_ref()
            .unwrap(),
        &views
    ));
    assert_eq!(views.by_selector.len(), 1);
    let selected = select(&known, source, &mut context)?.expect("prepared actual predicate item");
    assert_eq!(selected.selection.rows.as_slice(), &[1]);
    assert_eq!(
        context
            .stored_indexes
            .as_ref()
            .unwrap()
            .selection_counts
            .builds,
        1
    );
    Ok(())
}

#[test]
fn every_possible_discriminator_column_is_prepared_once() -> Result<(), Failure> {
    use ryo_values::domain::plan_parts::types::{
        ColumnIndex, Comparison, Operand, QueryLiteral, Test,
    };
    let mut selector = (*program(2)).clone();
    selector.identity = Arc::new(crate::domain::table_model::types::SelectionIdentity {
        comparisons: vec![1, 2]
            .into_iter()
            .map(|column| Comparison {
                test: Test::Equal,
                left: Operand::Column(ColumnIndex(column)),
                right: Operand::Literal(QueryLiteral::Number(Integer(2))),
                origin: None,
            })
            .collect(),
        bindings: vec![],
    });
    let selector = Arc::new(selector);
    let views =
        super::prepare_views_with_schemas([Arc::clone(&selector)], &|_| &[CellKind::Number; 3])?;
    let plans = &views.by_selector[&(Arc::as_ptr(&selector) as usize)].equalities;
    assert_eq!(plans.len(), 2);
    for column in [ColumnIndex(1), ColumnIndex(2)] {
        let plan = &plans[&column];
        assert_eq!(
            plan.kind,
            crate::domain::derived_views::types::DerivedKind::Equality
        );
        let crate::domain::derived_views::types::ViewRequest::Equality { columns } = &plan.request
        else {
            panic!("wrong prepared family");
        };
        assert_eq!(columns, &vec![column]);
        assert!(plan.metadata.is_ok());
    }
    Ok(())
}

#[test]
fn typed_positions_are_admitted_then_moved_without_copying_their_buffer() {
    use crate::domain::derived_views::{collections::views::registration::registered, types::*};
    let selector = program(2);
    let plan = registered(DerivedKind::PredicateMatches)
        .prepare(
            &ViewRequest::PredicateMatches(selector),
            &[crate::domain::operation_schema::types::InputSchema {
                kinds: &[CellKind::Number; 3],
                tree: None,
            }],
        )
        .unwrap();
    let rows = vec![0, 2];
    let pointer = rows.as_ptr();
    let mut storage = DerivedStorage::RowIds {
        kind: DerivedKind::PredicateMatches,
        rows,
    };
    let moved = super::take_positions(&plan, &mut storage, 3, 3).unwrap();
    assert_eq!(moved, [0, 2]);
    assert_eq!(pointer, moved.as_ptr());
    let mut invalid = DerivedStorage::RowIds {
        kind: DerivedKind::PredicateMatches,
        rows: vec![0, 3],
    };
    assert!(matches!(
        super::take_positions(&plan, &mut invalid, 3, 3),
        Err(OperatingError::ShapeMismatch)
    ));
    let mut wrong = DerivedStorage::RowIds {
        kind: DerivedKind::DistinctRepresentatives,
        rows: vec![0, 2],
    };
    assert!(matches!(
        super::take_positions(&plan, &mut wrong, 3, 3),
        Err(OperatingError::WrongOperation)
    ));
}

#[test]
fn empty_retention_counts_no_position_slots_and_invalid_group_is_not_allocation_failure()
-> Result<(), Failure> {
    let selector = program(2);
    let mut cache = new(&[Relation::Function], &[], LIMIT).unwrap();
    cache.selectors = vec![Arc::clone(&selector), Arc::clone(&selector)];
    let mut prepared =
        super::prepare_views_with_schemas([Arc::clone(&selector)], &|_| &[CellKind::Number; 3])?;
    let descriptor = Arc::clone(&prepared.by_selector[&(Arc::as_ptr(&selector) as usize)]);
    super::install_views(&mut cache, Arc::clone(&prepared));
    let columns = [
        FactColumn::default(),
        FactColumn::default(),
        FactColumn::default(),
    ];
    let source = ColumnsView {
        kinds: &[CellKind::Number; 3],
        columns: &columns,
    };
    let rows = Arc::new(Vec::new());
    let before = cache.bytes;
    assert!(matches!(
        super::admit(&mut cache, &selector, &descriptor, source, 0, &rows)?,
        super::Admission::Retained
    ));
    assert_eq!(cache.retained_positions, 0);
    assert!(cache.bytes > before && cache.bytes <= LIMIT);
    let mut fresh = new(&[Relation::Function], &[], LIMIT).unwrap();
    assert_eq!(fresh.retained_positions, 0);
    fresh.selectors = vec![Arc::clone(&selector), Arc::clone(&selector)];
    prepared =
        super::prepare_views_with_schemas([Arc::clone(&selector)], &|_| &[CellKind::Number; 3])?;
    Arc::get_mut(&mut prepared).unwrap().groups = 0;
    let descriptor = Arc::clone(&prepared.by_selector[&(Arc::as_ptr(&selector) as usize)]);
    super::install_views(&mut fresh, prepared);
    assert!(matches!(
        super::admit(&mut fresh, &selector, &descriptor, source, 0, &rows),
        Err(OperatingError::NotRun)
    ));
    assert_eq!(
        fresh.selection_counts.refusals, 0,
        "invalid preparation is not an allocation or payload-budget refusal"
    );
    Ok(())
}
