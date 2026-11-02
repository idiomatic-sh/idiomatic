//! Required sharing and optional retention are independent ownership decisions.
use super::*;
use crate::application::file_dag::derived::indexes;
use crate::domain::{
    derived_views::{
        service,
        types::{MembershipSpace, ViewRequest},
    },
    operation_schema::types::InputSchema,
    operator::contract::views::BorrowedRows,
    table_model::{service::push_row, types::Table},
};
use ryo_language::domain::relations::types::{CellKind, Integer, Value};
use ryo_values::domain::plan_parts::types::ColumnIndex;

fn source() -> Table {
    let mut table = Table {
        kinds: vec![CellKind::Number],
        ..Table::default()
    };
    for value in [3, 1, 3] {
        push_row(&mut table, &[Value::Number(Integer(value))]);
    }
    table
}

fn plan() -> Arc<DerivedContract> {
    Arc::new(
        service::prepare(
            DerivedKind::PartitionOrder,
            &ViewRequest::PartitionOrder {
                keys: vec![ColumnIndex(0)],
            },
            &[InputSchema {
                kinds: &[CellKind::Number],
                tree: None,
            }],
        )
        .unwrap(),
    )
}

fn membership_plan() -> Arc<DerivedContract> {
    Arc::new(
        service::prepare(
            DerivedKind::Membership,
            &ViewRequest::Membership {
                column: ColumnIndex(0),
            },
            &[InputSchema {
                kinds: &[CellKind::Number],
                tree: None,
            }],
        )
        .unwrap(),
    )
}

fn acquire_pair_membership(
    owner: &DerivedOwner,
    plan: &Arc<DerivedContract>,
    source: &Table,
    populations: [PopulationId; 2],
    scratch: &mut DerivedScratch,
    cache: &mut FileIndexes,
) -> Arc<DerivedStorage> {
    owner
        .acquire(
            plan,
            &populations,
            ViewContext::default(),
            DerivedInputs::Membership {
                rows: BorrowedRows::Computed(source),
                space: MembershipSpace::Values,
            },
            scratch,
            Some(cache),
        )
        .unwrap()
}

fn acquire(
    owner: &DerivedOwner,
    plan: &Arc<DerivedContract>,
    source: &Table,
    population: PopulationId,
    context: ViewContext,
    scratch: &mut DerivedScratch,
    cache: Option<&mut FileIndexes>,
) -> Arc<DerivedStorage> {
    let rows = [BorrowedRows::Computed(source)];
    owner
        .acquire(
            plan,
            &[population],
            context,
            DerivedInputs::Rows(&rows),
            scratch,
            cache,
        )
        .unwrap()
}

#[test]
fn pinned_population_shares_even_with_zero_entry_and_retained_budget() {
    let owner = DerivedOwner::new(0);
    let population = PopulationId(7);
    owner.pin(population);
    let source = source();
    let plan = plan();
    let mut scratch = DerivedScratch::default();
    let mut cache = indexes::new(&[], &[], 4096).unwrap();
    cache.limit = 0;

    let first = acquire(
        &owner,
        &plan,
        &source,
        population,
        ViewContext::default(),
        &mut scratch,
        Some(&mut cache),
    );
    let second = acquire(
        &owner,
        &plan,
        &source,
        population,
        ViewContext::default(),
        &mut scratch,
        Some(&mut cache),
    );
    assert!(Arc::ptr_eq(&first, &second));
    assert_eq!(owner.counts().0, 1);
    assert_eq!(owner.counts().1, 1);
    assert_eq!(owner.retained_bytes(), 0);
    assert!(owner.working_bytes() > 0);
    assert_eq!(owner.working_bytes(), owner.peak_working_bytes());
    owner.release(population);
    assert_eq!(owner.working_bytes(), 0);
    assert_eq!(owner.released_entries(), 1);
}

#[test]
fn expired_weak_entry_is_replaced_without_retaining_stale_storage() {
    let owner = DerivedOwner::new(1);
    let source = source();
    let plan = plan();
    let mut scratch = DerivedScratch::default();
    let first = acquire(
        &owner,
        &plan,
        &source,
        PopulationId(1),
        ViewContext::default(),
        &mut scratch,
        None,
    );
    let weak = Arc::downgrade(&first);
    drop(first);
    assert!(weak.upgrade().is_none());
    let second = acquire(
        &owner,
        &plan,
        &source,
        PopulationId(1),
        ViewContext::default(),
        &mut scratch,
        None,
    );
    assert!(weak.upgrade().is_none());
    assert_eq!(owner.counts().0, 2);
    assert_eq!(owner.counts().1, 0);
    assert_eq!(owner.state.borrow().entries.len(), 1);
    drop(second);
}

#[test]
fn optional_misses_do_not_accumulate_identity_metadata() {
    let owner = DerivedOwner::new(0);
    let source = source();
    let plan = plan();
    let mut scratch = DerivedScratch::default();
    let initial_metadata = owner.metadata_bytes();
    for identity in 0..256 {
        drop(acquire(
            &owner,
            &plan,
            &source,
            PopulationId(identity),
            ViewContext::default(),
            &mut scratch,
            None,
        ));
    }
    assert_eq!(owner.state.borrow().entries.len(), 0);
    assert_eq!(owner.state.borrow().dependents.len(), 0);
    assert_eq!(owner.metadata_bytes(), initial_metadata);
    assert_eq!(owner.counts(), (256, 0, 256));
}

#[test]
fn release_drops_only_the_dependent_required_view() {
    let owner = DerivedOwner::new(0);
    let first_population = PopulationId(1);
    let second_population = PopulationId(2);
    owner.pin(first_population);
    owner.pin(second_population);
    let source = source();
    let plan = plan();
    let mut scratch = DerivedScratch::default();
    let first = acquire(
        &owner,
        &plan,
        &source,
        first_population,
        ViewContext::default(),
        &mut scratch,
        None,
    );
    let second = acquire(
        &owner,
        &plan,
        &source,
        second_population,
        ViewContext::default(),
        &mut scratch,
        None,
    );
    assert!(!Arc::ptr_eq(&first, &second));
    let one_view_bytes = owner.working_bytes() / 2;
    owner.release(first_population);
    assert_eq!(owner.released_entries(), 1);
    assert_eq!(owner.working_bytes(), one_view_bytes);
    let again = acquire(
        &owner,
        &plan,
        &source,
        second_population,
        ViewContext::default(),
        &mut scratch,
        None,
    );
    assert!(Arc::ptr_eq(&second, &again));
    assert_eq!(owner.counts(), (2, 1, 2));
    owner.release(second_population);
    assert_eq!(owner.working_bytes(), 0);
    assert_eq!(owner.released_entries(), 2);
}

#[test]
fn two_population_view_survives_either_release_and_repin_without_duplicate_dependencies() {
    let owner = DerivedOwner::new(0);
    let populations = [PopulationId(1), PopulationId(2)];
    let source = source();
    let plan = membership_plan();
    let mut scratch = DerivedScratch::default();
    let mut cache = indexes::new(&[], &[], 4096).unwrap();
    let metadata_charge = cache.bytes;
    cache.limit = 0;
    let mut stable_metadata = None;

    for _ in 0..64 {
        owner.pin(populations[0]);
        owner.pin(populations[1]);
        let first = acquire_pair_membership(
            &owner,
            &plan,
            &source,
            populations,
            &mut scratch,
            &mut cache,
        );
        let weak = Arc::downgrade(&first);
        drop(first);

        owner.release(populations[0]);
        assert!(weak.upgrade().is_some());
        assert!(owner.working_bytes() > 0);
        owner.pin(populations[0]);
        let after_first = acquire_pair_membership(
            &owner,
            &plan,
            &source,
            populations,
            &mut scratch,
            &mut cache,
        );
        assert!(Arc::ptr_eq(&after_first, &weak.upgrade().unwrap()));
        drop(after_first);

        owner.release(populations[1]);
        assert!(weak.upgrade().is_some());
        let after_second = acquire_pair_membership(
            &owner,
            &plan,
            &source,
            populations,
            &mut scratch,
            &mut cache,
        );
        assert!(Arc::ptr_eq(&after_second, &weak.upgrade().unwrap()));
        drop(after_second);

        owner.release(populations[0]);
        assert!(weak.upgrade().is_none());
        assert_eq!(owner.working_bytes(), 0);
        assert_eq!(
            cache.bytes, metadata_charge,
            "zero payload budget never removes existing owner metadata"
        );
        let state = owner.state.borrow();
        assert_eq!(state.entries.len(), 1);
        assert_eq!(state.dependents[&populations[0]].len(), 1);
        assert_eq!(state.dependents[&populations[1]].len(), 1);
        drop(state);
        match stable_metadata {
            None => stable_metadata = Some(owner.metadata_bytes()),
            Some(bytes) => assert_eq!(owner.metadata_bytes(), bytes),
        }
    }
    assert_eq!(owner.counts().0, 64);
}

#[test]
fn population_and_semantic_context_are_part_of_the_view_identity() {
    let owner = DerivedOwner::new(0);
    let first_population = PopulationId(1);
    let second_population = PopulationId(2);
    owner.pin(first_population);
    owner.pin(second_population);
    let source = source();
    let plan = plan();
    let mut scratch = DerivedScratch::default();
    let first = acquire(
        &owner,
        &plan,
        &source,
        first_population,
        ViewContext::default(),
        &mut scratch,
        None,
    );
    let other_population = acquire(
        &owner,
        &plan,
        &source,
        second_population,
        ViewContext::default(),
        &mut scratch,
        None,
    );
    let other_context = acquire(
        &owner,
        &plan,
        &source,
        first_population,
        ViewContext {
            candidate: Some(9),
            membership_order: Some(2),
        },
        &mut scratch,
        None,
    );
    assert!(!Arc::ptr_eq(&first, &other_population));
    assert!(!Arc::ptr_eq(&first, &other_context));
    assert!(!Arc::ptr_eq(&other_population, &other_context));
    assert_eq!(owner.counts(), (3, 0, 3));
}
