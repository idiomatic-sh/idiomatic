//! Admission and once-charged cache policy are exercised under their application owner.
use crate::application::file_dag::derived::owner::selection::fixtures::SelectionFixture;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::OperatorContext;
use crate::domain::stored_selection::fixtures::program;
use crate::domain::table_scratch::types::OperatorScratch;
use ryo_derived::domain::contract::types::DerivedView;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::service::to_bits;
use ryo_relations::domain::relations::types::{
    CellKind, ColumnsView, FactColumn, Integer, Relation, Value as Cell,
};
use ryo_values::domain::files::types::SourceView;
use ryo_values::domain::rows::types::{RowCount, Table};
use std::sync::Arc;
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
        matcher: crate::domain::matching::types::PatternMatcher(&ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl),
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
        &selected.selection.rows.0,
        &repeated.selection.rows.0
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
        operator::contract::views::{PredicateProgram},
        stored_selection::types::Selector,
        stored_selection::types::SelectionIdentity,
        table_model::types::Verdict,
    };
use crate::domain::table_model::types::RepresentationInput as BorrowedRows;
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
        distinct: ryo_values::domain::plan_parts::types::RepeatedRows::Kept,
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
        crate::application::file_dag::derived::owner::index::service::LIMIT,
        "capacity refusal is tested with the normal byte budget"
    );
    let table = Table {
        kinds: vec![],
        cells: vec![],
        count: RowCount(super::super::types::MAX_RETAINED_POSITION_CAPACITY + 1),
    };
    let owner = fixture.owner();
    let publication = owner.computed(table.clone())?;
    let held = owner.resolve(publication.as_ref(), &|_| Err(OperatingError::NotRead))?;
    let rows = BorrowedRows::from(&held);
    let proof = owner.admit(publication.as_ref(), rows)?;
    let plan = Arc::new(crate::application::file_dag::derived::prepare::view(
        (std::sync::Arc::new(
            crate::outbound::derived::factory::semantics(),
        ) as std::sync::Arc<dyn crate::outbound::derived::interface::DerivedPort>),
        &ViewRequest::PredicateMatches(selector),
        &[InputShape {
            kinds: &[],
            tree: None,
        }],
    )?);
    let handle = owner.derived().acquire_stored(
        &plan,
        &owner.derived().admit_view(&plan, &owner, &[&proof]).unwrap(),
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
    let DerivedView::RowIds(ryo_derived::domain::positions::types::RowSlots(rows)) = handle.view()? else {
        panic!("predicate view")
    };
    assert_eq!(
        rows.len(),
        super::super::types::MAX_RETAINED_POSITION_CAPACITY + 1
    );
    assert_eq!(rows.first(), Some(&0));
    assert_eq!(
        rows.last(),
        Some(&(super::super::types::MAX_RETAINED_POSITION_CAPACITY as u32))
    );
    let after = fixture.snapshot(None);
    assert_eq!(after.selections, 0);
    assert_eq!(after.retained_positions, 0);
    assert_eq!(after.bytes, before.bytes);
    assert_eq!(owner.derived().counts().2, 1);
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
        matcher: crate::domain::matching::types::PatternMatcher(&ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl),
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
        ColumnIndex, PlanComparison, Operand, QueryLiteral, Test,
    };
    let mut selector = (*program(2)).clone();
    selector.identity = Arc::new(crate::domain::stored_selection::types::SelectionIdentity {
        comparisons: vec![1, 2]
            .into_iter()
            .map(|column| PlanComparison {
                test: Test::Equal,
                left: Operand::Column(ColumnIndex(column)),
                right: Operand::Literal(QueryLiteral::Number(Integer(2))),
                origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
            })
            .collect(),
        bindings: vec![],
    });
    let selector = Arc::new(selector);
    let views = super::super::preparation::service::prepare_views_with_schemas(
        [(selector).clone()],
        &|_| &[CellKind::Number; 3],
        (std::sync::Arc::new(
            crate::outbound::derived::factory::semantics(),
        ) as std::sync::Arc<dyn crate::outbound::derived::interface::DerivedPort>),
    )?;
    let plans = &views.by_selector[&crate::domain::derived_views::types::SelectorAddress(Arc::as_ptr(&selector) as usize)].equalities;
    assert_eq!(plans.len(), 2);
    for column in [ColumnIndex(1), ColumnIndex(2)] {
        let plan = &plans[&column];
        assert_eq!(
            plan.kind,
            ryo_derived::domain::request::types::DerivedKind::Equality
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
