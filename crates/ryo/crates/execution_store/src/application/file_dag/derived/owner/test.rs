//! Required sharing and optional retention are independent ownership decisions.
use super::*;

mod release;
use crate::application::file_dag::derived::owner::index::service as indexes;
use crate::application::file_dag::derived::owner::index::types::ByteLimit;
use crate::application::file_dag::relations::{RelationOwner, handle::service::PublishedInput};
use ryo_derived::domain::contract::types::DerivedView;
use ryo_execution_domain::domain::table_model::types::RepresentationInput;
use ryo_relations::domain::relations::types::{CellKind, Integer, Value};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use {
    ryo_execution_domain::domain::operation_schema::types::InputShape,
    ryo_values::domain::rows::service::push_row,
};

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
        crate::application::file_dag::derived::prepare::view(
            std::sync::Arc::new(crate::application::file_dag::fixtures::derived()),
            &ViewRequest::PartitionOrder {
                keys: vec![ColumnIndex(0)],
            },
            &[InputShape {
                kinds: &[CellKind::Number],
                tree: None,
            }],
        )
        .unwrap(),
    )
}

fn owner(entries: EntryLimit) -> RelationOwner {
    let mut owner = RelationOwner::new(
        crate::application::file_dag::relations::types::ReuseEligibility::Eligible,
        crate::application::file_dag::fixtures::program_derived_port(),
    );
    owner.derived_mut().max_entries = entries;
    owner
}
fn population(owner: &RelationOwner, publication: &PublishedInput) -> PopulationId {
    let held = owner
        .resolve(publication.as_ref(), &|_| Err(OperatingError::NotRead))
        .unwrap();
    owner
        .admit(publication.as_ref(), RepresentationInput::from(&held))
        .unwrap()
        .identity(owner)
        .unwrap()
        .population
}
fn pair_plan() -> Arc<DerivedContract> {
    Arc::new(
        crate::application::file_dag::derived::prepare::view(
            crate::application::file_dag::fixtures::program_derived_port(),
            &ViewRequest::OrderedKeys {
                left_on: vec![ColumnIndex(0)],
                right_on: vec![ColumnIndex(0)],
                left_order: ColumnIndex(0),
                right_order: ColumnIndex(0),
            },
            &[
                InputShape {
                    kinds: &[CellKind::Number],
                    tree: None,
                },
                InputShape {
                    kinds: &[CellKind::Number],
                    tree: None,
                },
            ],
        )
        .unwrap(),
    )
}
fn acquire_pair(
    owner: &RelationOwner,
    plan: &Arc<DerivedContract>,
    publications: &[PublishedInput; 2],
    scratch: &mut DerivedScratch,
) -> acquisition::admission::service::AcquiredView {
    let left = owner
        .resolve(publications[0].as_ref(), &|_| Err(OperatingError::NotRead))
        .unwrap();
    let right = owner
        .resolve(publications[1].as_ref(), &|_| Err(OperatingError::NotRead))
        .unwrap();
    let left = owner
        .admit(publications[0].as_ref(), RepresentationInput::from(&left))
        .unwrap();
    let right = owner
        .admit(publications[1].as_ref(), RepresentationInput::from(&right))
        .unwrap();
    let admitted = owner
        .derived()
        .admit_view(plan, owner, &[&left, &right])
        .unwrap();
    let symbols = ryo_language::domain::symbols::types::Symbols::default();
    let texts = ryo_execution_domain::domain::table_texts::types::Texts {
        symbols: &symbols,
        source: ryo_values::domain::files::types::SourceView(&[]),
        paths: &[],
        arenas: &[],
    };
    owner
        .derived()
        .acquire_stored(
            plan,
            &admitted,
            ViewContext::default(),
            DerivedInputs::Text {
                rows: &[left.rows(), right.rows()],
                texts: &texts,
            },
            scratch,
            None,
        )
        .unwrap()
}
fn acquire(
    owner: &RelationOwner,
    plan: &Arc<DerivedContract>,
    publication: &PublishedInput,
    context: ViewContext,
    scratch: &mut DerivedScratch,
) -> acquisition::admission::service::AcquiredView {
    let held = owner
        .resolve(publication.as_ref(), &|_| Err(OperatingError::NotRead))
        .unwrap();
    let proof = owner
        .admit(publication.as_ref(), RepresentationInput::from(&held))
        .unwrap();
    let admitted = owner.derived().admit_view(plan, owner, &[&proof]).unwrap();
    owner
        .derived()
        .acquire_stored(
            plan,
            &admitted,
            context,
            DerivedInputs::Rows(&[proof.rows()]),
            scratch,
            None,
        )
        .unwrap()
}

#[test]
fn pinned_population_shares_even_with_zero_entry_and_retained_budget() {
    let relation_owner = owner(EntryLimit(0));
    let owner = relation_owner.derived();
    let publication = relation_owner.computed(source()).unwrap();
    let population = population(&relation_owner, &publication);
    owner.pin(population);
    let plan = plan();
    let mut scratch = DerivedScratch::default();
    let mut cache = indexes::new(&[], &[], ByteLimit(4096)).unwrap();
    cache.limit.0 = 0;
    owner.backend.borrow_mut().replace(cache);

    let first = acquire(
        &relation_owner,
        &plan,
        &publication,
        ViewContext::default(),
        &mut scratch,
    );
    let second = acquire(
        &relation_owner,
        &plan,
        &publication,
        ViewContext::default(),
        &mut scratch,
    );
    assert_eq!(first.storage_identity(), second.storage_identity());
    let DerivedView::PartitionOrder(rows) = first.view().unwrap() else {
        panic!("prepared ordering view")
    };
    assert_eq!(rows.iter().map(|row| row.0).collect::<Vec<_>>(), [1, 0, 2]);
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
    let relation_owner = owner(EntryLimit(1));
    let owner = relation_owner.derived();
    let publication = relation_owner.computed(source()).unwrap();
    let plan = plan();
    let mut scratch = DerivedScratch::default();
    let first = acquire(
        &relation_owner,
        &plan,
        &publication,
        ViewContext::default(),
        &mut scratch,
    );
    let weak = Arc::downgrade(first.storage());
    drop(first);
    assert!(weak.upgrade().is_none());
    let second = acquire(
        &relation_owner,
        &plan,
        &publication,
        ViewContext::default(),
        &mut scratch,
    );
    assert!(weak.upgrade().is_none());
    assert_eq!(owner.counts().0, 2);
    assert_eq!(owner.counts().1, 0);
    assert_eq!(owner.state.borrow().entries.len(), 1);
    drop(second);
}

#[test]
fn optional_misses_do_not_accumulate_identity_metadata() {
    let relation_owner = owner(EntryLimit(0));
    let owner = relation_owner.derived();
    let plan = plan();
    let mut scratch = DerivedScratch::default();
    let initial_metadata = owner.metadata_bytes();
    for _ in 0..256 {
        let publication = relation_owner.computed(source()).unwrap();
        drop(acquire(
            &relation_owner,
            &plan,
            &publication,
            ViewContext::default(),
            &mut scratch,
        ));
    }
    assert_eq!(owner.state.borrow().entries.len(), 0);
    assert_eq!(owner.state.borrow().dependents.len(), 0);
    assert_eq!(owner.metadata_bytes(), initial_metadata);
    assert_eq!(
        owner.counts(),
        BuildCounts(BuildCount(256), HitCount(0), RefusalCount(256))
    );
}

#[test]
fn population_and_semantic_context_are_part_of_the_view_identity() {
    let relation_owner = owner(EntryLimit(0));
    let owner = relation_owner.derived();
    let first_publication = relation_owner.computed(source()).unwrap();
    let second_publication = relation_owner.computed(source()).unwrap();
    let first_population = population(&relation_owner, &first_publication);
    let second_population = population(&relation_owner, &second_publication);
    owner.pin(first_population);
    owner.pin(second_population);
    let plan = plan();
    let mut scratch = DerivedScratch::default();
    let first = acquire(
        &relation_owner,
        &plan,
        &first_publication,
        ViewContext::default(),
        &mut scratch,
    );
    let other_population = acquire(
        &relation_owner,
        &plan,
        &second_publication,
        ViewContext::default(),
        &mut scratch,
    );
    let other_context = acquire(
        &relation_owner,
        &plan,
        &first_publication,
        ViewContext {
            membership_order: Some(super::types::MembershipOrder(2)),
            ..ViewContext::for_predicate(&plan)
        },
        &mut scratch,
    );
    assert_ne!(
        first.storage_identity(),
        other_population.storage_identity()
    );
    assert_ne!(first.storage_identity(), other_context.storage_identity());
    assert_ne!(
        other_population.storage_identity(),
        other_context.storage_identity()
    );
    assert_eq!(
        owner.counts(),
        BuildCounts(BuildCount(3), HitCount(0), RefusalCount(3))
    );
}

use ryo_values::domain::rows::types::Table;
