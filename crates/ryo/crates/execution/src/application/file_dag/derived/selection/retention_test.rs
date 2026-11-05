//! Admission and once-charged cache policy are exercised under their application owner.
use crate::application::file_dag::derived::selection::fixtures::SelectionFixture;
use ryo_derived::domain::contract::types::DerivedView;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::{
    service::to_bits,
    types::{CellKind, ColumnsView, FactColumn, Integer, Relation, Value as Cell},
};
use std::sync::Arc;
use {
    crate::domain::operating::errors::OperatingError,
    crate::domain::operating::types::OperatorContext,
    crate::domain::stored_selection::fixtures::program,
    crate::domain::table_scratch::types::OperatorScratch,
    ryo_values::domain::files::types::SourceView,
};
type Failure = OperatingError;
/// Entries append into spare owned capacity instead of moving every retained entry.
#[test]
fn retained_selections_keep_spare_owner_capacity_and_bounded_reacquisition() -> Result<(), Failure>
{
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
    let active = vec![first.clone(), first.clone(), second.clone(), second.clone()];
    let fixture = SelectionFixture::new(
        &[Relation::Function],
        &[],
        active.clone(),
        active,
        source.kinds,
    )?;
    fixture.reserve_spare_selections(8);
    let initial = fixture.snapshot(Some(&first));
    assert!(initial.selections_capacity >= 8);
    let storage_capacity = initial.selections_capacity;
    let mut symbols = Symbols::default();
    let mut memo = Vec::new();
    let mut scratch = OperatorScratch::default();
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
        memo: &mut memo,
        scratch: &mut scratch,
    };
    let selected = fixture
        .select(&first, source, &mut context)?
        .expect("first result");
    let after_first = fixture.snapshot(Some(&first));
    let descriptor = after_first
        .descriptor_address
        .expect("actual selector descriptor");
    assert_eq!(
        descriptor,
        after_first.prepared_descriptor_address.unwrap(),
        "actual selector acquisition shares request descriptor allocation"
    );
    let predicate_cells = after_first.counts.predicate_cells;
    let repeated = fixture
        .select(&first, source, &mut context)?
        .expect("shared repeated result");
    assert!(Arc::ptr_eq(
        &selected.selection.rows,
        &repeated.selection.rows
    ));
    let after_reuse = fixture.snapshot(Some(&first));
    assert_eq!(after_reuse.descriptor_address, Some(descriptor));
    assert_eq!(after_reuse.counts.predicate_cells, predicate_cells);
    let selected_next = fixture
        .select(&second, source, &mut context)?
        .expect("second result");
    assert_ne!(selected.selection.rows, selected_next.selection.rows);
    let retained = fixture.snapshot(Some(&first));
    assert_eq!(retained.selections, 2);
    assert_eq!(
        retained.retained_positions,
        retained.retained_positions_capacity
    );
    assert_eq!(retained.selections_capacity, storage_capacity);
    assert!(retained.bytes > initial.bytes && retained.bytes <= retained.limit);
    assert!(retained.owner_metadata_bytes >= initial.owner_metadata_bytes);
    for _ in 0..32 {
        for program in [&first, &second] {
            drop(
                fixture
                    .select(program, source, &mut context)?
                    .expect("retained result"),
            );
        }
    }
    let repeated = fixture.snapshot(Some(&first));
    assert_eq!(repeated.selections_capacity, retained.selections_capacity);
    assert_eq!(repeated.owner_metadata_bytes, retained.owner_metadata_bytes);
    assert_eq!(repeated.optional_bytes, retained.optional_bytes);
    assert_eq!(
        repeated.retained_positions_capacity,
        retained.retained_positions_capacity
    );
    assert_eq!(
        repeated.counts.predicate_cells,
        retained.counts.predicate_cells
    );
    Ok(())
}

/// A result larger than the selection allowance remains a valid required view,
/// while its optional retention is refused by the common owner.
#[test]
fn oversized_selection_refuses_retention_without_charging_the_cache() -> Result<(), Failure> {
    use crate::application::file_dag::derived::owner::ViewContext;
    use crate::domain::{
        derived_views::types::*,
        operation_schema::types::InputShape,
        operator::contract::{types::PredicateProgram, views::BorrowedRows},
        stored_selection::types::Selector,
        table_model::types::{SelectionIdentity, Verdict},
    };
    use ryo_relations::domain::relations::types::Value;

    struct AlwaysHolds;
    impl PredicateProgram for AlwaysHolds {
        fn holds(&mut self, _: &[Value], _: &Symbols) -> Result<Verdict, OperatingError> {
            Ok(Verdict::Holds)
        }
    }
    let selector = Arc::new(Selector {
        relation: Relation::Function,
        identity: Arc::new(SelectionIdentity {
            comparisons: vec![],
            bindings: vec![],
        }),
        columns: vec![],
        distinct: false,
    });
    let fixture = SelectionFixture::new(
        &[Relation::Function],
        &[],
        vec![selector.clone(), selector.clone()],
        [selector.clone()],
        &[],
    )?;
    let before = fixture.snapshot(None);
    assert_eq!(
        before.limit,
        super::super::indexes::LIMIT,
        "capacity refusal is tested with the normal byte budget"
    );
    let table = Table {
        kinds: vec![],
        cells: vec![],
        count: RowCount(super::MAX_RETAINED_POSITION_CAPACITY + 1),
    };
    let owner = fixture.owner();
    let publication = owner.computed(table.clone())?;
    let held = owner.resolve(publication.as_ref(), &|_| Err(OperatingError::NotRead))?;
    let rows = BorrowedRows::from(&held);
    let proof = owner.admit(publication.as_ref(), rows)?;
    let (population, _) = proof.identity(owner)?;
    let plan = Arc::new(crate::application::file_dag::derived::prepare::view(
        (std::sync::Arc::new(
            ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new(),
        ) as std::sync::Arc<dyn crate::outbound::derived::interface::DerivedPort>),
        &ViewRequest::PredicateMatches(selector),
        &[InputShape {
            kinds: &[],
            tree: None,
        }],
    )?);
    let handle = owner.derived.acquire_stored(
        &plan,
        &[population],
        ViewContext::default(),
        DerivedInputs::Predicate {
            rows: proof.rows(),
            candidates: None,
            program: &mut AlwaysHolds,
            symbols: &Symbols::default(),
        },
        &mut DerivedScratch::default(),
        None,
    )?;
    let DerivedView::RowIds(rows) = handle.view()? else {
        panic!("predicate view")
    };
    assert_eq!(rows.len(), super::MAX_RETAINED_POSITION_CAPACITY + 1);
    assert_eq!(rows.first(), Some(&0));
    assert_eq!(
        rows.last(),
        Some(&(super::MAX_RETAINED_POSITION_CAPACITY as u32))
    );
    let after = fixture.snapshot(None);
    assert_eq!(after.selections, 0);
    assert_eq!(after.retained_positions, 0);
    assert_eq!(after.bytes, before.bytes);
    assert_eq!(owner.derived.counts().2, 1);
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
    let fixture = SelectionFixture::new(
        &[Relation::Function],
        &[],
        vec![known.clone()],
        [known.clone()],
        source.kinds,
    )?;
    let before = fixture.snapshot(Some(&known));
    let (mut symbols, mut memo, mut scratch) =
        (Symbols::default(), Vec::new(), OperatorScratch::default());
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
        memo: &mut memo,
        scratch: &mut scratch,
    };
    assert!(fixture.select(&unknown, source, &mut context)?.is_none());
    let after_unknown = fixture.snapshot(Some(&known));
    assert_eq!(after_unknown.counts.builds, 0);
    assert_eq!(
        after_unknown.prepared_views_address,
        before.prepared_views_address
    );
    assert_eq!(after_unknown.prepared_selectors, 1);
    let selected = fixture
        .select(&known, source, &mut context)?
        .expect("prepared actual predicate item");
    assert_eq!(selected.selection.rows.as_slice(), &[1]);
    assert_eq!(fixture.snapshot(Some(&known)).counts.builds, 1);
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
    let views = super::prepare_views_with_schemas(
        [(selector).clone()],
        &|_| &[CellKind::Number; 3],
        (std::sync::Arc::new(
            ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new(),
        ) as std::sync::Arc<dyn crate::outbound::derived::interface::DerivedPort>),
    )?;
    let plans = &views.by_selector[&(Arc::as_ptr(&selector) as usize)].equalities;
    assert_eq!(plans.len(), 2);
    for column in [ColumnIndex(1), ColumnIndex(2)] {
        let plan = &plans[&column];
        assert_eq!(
            plan.kind,
            ryo_derived::domain::contract::types::DerivedKind::Equality
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
fn typed_positions_share_the_owner_acquired_buffer_and_refuse_wrong_views() -> Result<(), Failure> {
    use crate::application::file_dag::{derived::owner::ViewContext, relations::RelationOwner};
    use ryo_values::domain::plan_parts::types::ColumnIndex;
    use {
        crate::domain::derived_views::types::*, crate::domain::operation_schema::types::InputShape,
        crate::domain::operator::contract::views::BorrowedRows,
        ryo_values::domain::rows::service::push_row,
    };

    let mut table = Table {
        kinds: vec![CellKind::Number],
        ..Table::default()
    };
    for value in [1, 1, 2] {
        push_row(&mut table, &[Cell::Number(Integer(value))]);
    }
    let schema = [InputShape {
        kinds: &table.kinds,
        tree: None,
    }];
    let distinct = Arc::new(crate::application::file_dag::derived::prepare::view(
        (std::sync::Arc::new(
            ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new(),
        ) as std::sync::Arc<dyn crate::outbound::derived::interface::DerivedPort>),
        &ViewRequest::DistinctRepresentatives {
            columns: vec![ColumnIndex(0)],
        },
        &schema,
    )?);
    let order = Arc::new(crate::application::file_dag::derived::prepare::view(
        (std::sync::Arc::new(
            ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new(),
        ) as std::sync::Arc<dyn crate::outbound::derived::interface::DerivedPort>),
        &ViewRequest::PartitionOrder {
            keys: vec![ColumnIndex(0)],
        },
        &schema,
    )?);
    let owner = RelationOwner::new(
        true,
        (std::sync::Arc::new(
            ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new(),
        ) as std::sync::Arc<dyn crate::outbound::derived::interface::DerivedPort>),
    );
    let publication = owner.computed(table.clone())?;
    let held = owner.resolve(publication.as_ref(), &|_| Err(OperatingError::NotRead))?;
    let rows = BorrowedRows::from(&held);
    let proof = owner.admit(publication.as_ref(), rows)?;
    let (population, _) = proof.identity(&owner)?;
    let acquired = owner.derived.acquire_stored(
        &distinct,
        &[population],
        ViewContext::default(),
        DerivedInputs::Distinct {
            rows: proof.rows(),
            positions: &[0, 1, 2],
        },
        &mut DerivedScratch::default(),
        None,
    )?;
    let DerivedView::RowIds(actual) = acquired.view()? else {
        panic!("registered distinct view")
    };
    let original_buffer = actual.as_ptr();
    let shared = super::shared_positions(&acquired, 3, 3)?;
    assert_eq!(shared.as_slice(), &[0, 2]);
    assert_eq!(shared.as_ptr(), original_buffer, "no row-ID buffer copy");
    assert!(matches!(
        super::shared_positions(&acquired, 2, 3),
        Err(OperatingError::ShapeMismatch)
    ));
    assert!(matches!(
        super::shared_positions(&acquired, 3, 1),
        Err(OperatingError::ShapeMismatch)
    ));
    let wrong = owner.derived.acquire_stored(
        &order,
        &[population],
        ViewContext::default(),
        DerivedInputs::Rows(&[proof.rows()]),
        &mut DerivedScratch::default(),
        None,
    )?;
    assert!(matches!(
        super::shared_positions(&wrong, 3, 3),
        Err(OperatingError::WrongOperation)
    ));
    Ok(())
}

#[test]
fn empty_retention_counts_no_position_slots_and_invalid_group_is_not_allocation_failure()
-> Result<(), Failure> {
    let selector = program(2);
    let active = vec![selector.clone(), selector.clone()];
    let fixture = SelectionFixture::new(
        &[Relation::Function],
        &[],
        active.clone(),
        [selector.clone()],
        &[CellKind::Number; 3],
    )?;
    let columns = [
        FactColumn::default(),
        FactColumn::default(),
        FactColumn::default(),
    ];
    let source = ColumnsView {
        kinds: &[CellKind::Number; 3],
        columns: &columns,
    };
    let mut symbols = Symbols::default();
    let mut memo = Vec::new();
    let mut scratch = OperatorScratch::default();
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
        memo: &mut memo,
        scratch: &mut scratch,
    };
    let before = fixture.snapshot(None);
    let selected = fixture
        .select(&selector, source, &mut context)?
        .expect("complete empty result");
    assert!(selected.selection.rows.is_empty());
    let after = fixture.snapshot(None);
    assert_eq!(after.selections, 1);
    assert_eq!(after.retained_positions, 0);
    assert!(after.bytes > before.bytes && after.bytes <= after.limit);
    let fresh = SelectionFixture::new(
        &[Relation::Function],
        &[],
        active,
        [selector.clone()],
        source.kinds,
    )?;
    assert_eq!(fresh.snapshot(None).retained_positions, 0);
    fresh.corrupt_prepared_group_count();
    assert!(matches!(
        fresh.select(&selector, source, &mut context),
        Err(OperatingError::NotRun)
    ));
    assert_eq!(
        fresh.snapshot(None).counts.refusals,
        0,
        "invalid preparation is not an allocation or payload-budget refusal"
    );
    Ok(())
}

use ryo_values::domain::rows::types::{RowCount, Table};

use ryo_derived::domain::contract::types::DerivedKind;
