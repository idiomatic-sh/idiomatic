//! Required sharing and optional retention are independent ownership decisions.
use super::super::super::*;
use super::super::fixtures::{acquire, owner, plan, population, source};
use crate::cases::{Case, CaseName, Expect, Word, run};

use crate::application::file_dag::derived::owner::index::service as indexes;
use crate::application::file_dag::derived::owner::index::types::ByteLimit;
use ryo_derived::domain::contract::types::DerivedView;

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
            membership_order: Some(super::super::super::types::MembershipOrder(2)),
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

/// The case each row runs: one of the proofs below.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    PinnedPopulationSharesEvenWithZeroEntryAndRetainedBudget,
    ExpiredWeakEntryIsReplacedWithoutRetainingStaleStorage,
    OptionalMissesDoNotAccumulateIdentityMetadata,
    PopulationAndSemanticContextArePartOfTheViewIdentity,
}

type Input = Scenario;
type Value = ();
type Failure = String;

/// One row per proof: its name, and the scenario that runs it.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("pinned_population_shares_even_with_zero_entry_and_retained_budget"),
        word: Word::Succeeds,
        input: || Scenario::PinnedPopulationSharesEvenWithZeroEntryAndRetainedBudget,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("expired_weak_entry_is_replaced_without_retaining_stale_storage"),
        word: Word::Succeeds,
        input: || Scenario::ExpiredWeakEntryIsReplacedWithoutRetainingStaleStorage,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("optional_misses_do_not_accumulate_identity_metadata"),
        word: Word::Succeeds,
        input: || Scenario::OptionalMissesDoNotAccumulateIdentityMetadata,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("population_and_semantic_context_are_part_of_the_view_identity"),
        word: Word::Succeeds,
        input: || Scenario::PopulationAndSemanticContextArePartOfTheViewIdentity,
        expect: Expect::Value(()),
    },
];

/// Runs one scenario; its assertions are the case's expectation.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::PinnedPopulationSharesEvenWithZeroEntryAndRetainedBudget => {
            pinned_population_shares_even_with_zero_entry_and_retained_budget();
            Ok(())
        }
        Scenario::ExpiredWeakEntryIsReplacedWithoutRetainingStaleStorage => {
            expired_weak_entry_is_replaced_without_retaining_stale_storage();
            Ok(())
        }
        Scenario::OptionalMissesDoNotAccumulateIdentityMetadata => {
            optional_misses_do_not_accumulate_identity_metadata();
            Ok(())
        }
        Scenario::PopulationAndSemanticContextArePartOfTheViewIdentity => {
            population_and_semantic_context_are_part_of_the_view_identity();
            Ok(())
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
