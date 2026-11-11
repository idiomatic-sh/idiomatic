//! Release drops only the required views of the released population.
use super::super::super::*;
use crate::application::file_dag::derived::ledger::types::ByteCount;
use crate::application::file_dag::derived::owner::cache::types::CacheBackend;
use crate::application::file_dag::derived::population::service::PopulationId;
use crate::application::file_dag::derived::owner::acquisition::semantics::storage::DerivedStorage;
use crate::application::file_dag::derived::owner::footprint::service::storage_bytes;
use rustc_hash::FxHashMap;
use ryo_execution_domain::domain::{
    derived_views::types::{contract::value::DerivedContract, *},
    operating::errors::OperatingError,
};
use std::{
    cell::RefCell,
    sync::{Arc, Weak},
};
use crate::application::file_dag::derived::owner::types::{
    AdmissionOrder, BuildCount, BuildCounts, EntryLimit, FilePreparation, HitCount,
    IdentityCompatibility, MembershipOrder, MemoBudget, PinCount, PinState, PopulationCount,
    ReclaimOutlook, RefusalCount, ReleasedEntries, Room, ScopeId, ViewCount,
};
use crate::application::file_dag::derived::owner::graph_key::{GraphRequirement, GraphViewIdentity, RequirementIdentity};
use ryo_derived::domain::request::types::DerivedKind;
use super::super::super::service::*;
use super::super::super::types::*;
use super::super::fixtures::{acquire, acquire_pair, owner, pair_plan, plan, population, source};
use crate::application::file_dag::derived::owner::index::service as indexes;
use crate::application::file_dag::derived::owner::index::types::ByteLimit;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_relations::domain::relations::types::{Integer, Value as CellValue};

/// How the required views depend on the pinned populations.
#[derive(Clone, Copy, Debug)]
enum Dependence {
    /// Each of two populations has its own view of one plan; both are released in turn.
    OneViewEach,
    /// One view spans both populations; for `rounds` rounds both are pinned, released in turn and
    /// repinned.
    OneViewForBoth { rounds: u64 },
}

/// What releasing one of two populations, each with its own view, observed.
#[derive(Debug, PartialEq)]
struct OwnViews {
    distinct_storage: bool,
    released_after_first: ReleasedEntries,
    one_view_left: bool,
    second_reused: bool,
    counts: BuildCounts,
    working_after_both: usize,
    released_after_both: ReleasedEntries,
}

/// In how many of the rounds each property of the shared view held, and the views built.
#[derive(Debug, PartialEq)]
struct SharedView {
    survived_first_release: u64,
    working_after_first_release: u64,
    reused_after_first_repin: u64,
    survived_second_release: u64,
    reused_after_second_release: u64,
    dropped_after_both_released: u64,
    no_working_bytes_after_both: u64,
    metadata_charge_kept: u64,
    one_entry_one_dependent_each: u64,
    metadata_stable: u64,
    builds: BuildCount,
}

/// What the releases observed.
#[derive(Debug, PartialEq)]
enum Observed {
    OwnViews(OwnViews),
    SharedView(SharedView),
}

type Input = Dependence;
type Value = Observed;
type Failure = String;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("release_drops_only_the_dependent_required_view"),
        word: Word::Succeeds,
        input: || Dependence::OneViewEach,
        expect: Expect::Value(Observed::OwnViews(OwnViews {
            distinct_storage: true,
            released_after_first: ReleasedEntries(1),
            one_view_left: true,
            second_reused: true,
            counts: BuildCounts(BuildCount(2), HitCount(1), RefusalCount(2)),
            working_after_both: 0,
            released_after_both: ReleasedEntries(2),
        })),
    },
    Case {
        name: CaseName(
            "two_population_view_survives_either_release_and_repin_without_duplicate_dependencies",
        ),
        word: Word::Succeeds,
        input: || Dependence::OneViewForBoth { rounds: 64 },
        expect: Expect::Value(Observed::SharedView(SharedView {
            survived_first_release: 64,
            working_after_first_release: 64,
            reused_after_first_repin: 64,
            survived_second_release: 64,
            reused_after_second_release: 64,
            dropped_after_both_released: 64,
            no_working_bytes_after_both: 64,
            metadata_charge_kept: 64,
            one_entry_one_dependent_each: 64,
            metadata_stable: 64,
            builds: BuildCount(64),
        })),
    },
];

/// Pins the populations of two publications of one source, acquires their views and releases.
fn subject(input: Input) -> Result<Value, Failure> {
    Ok(match input {
        Dependence::OneViewEach => Observed::OwnViews(own_views()),
        Dependence::OneViewForBoth { rounds } => Observed::SharedView(shared_view(rounds)),
    })
}

fn own_views() -> OwnViews {
    let relation_owner = owner(EntryLimit(0));
    let owner = relation_owner.derived();
    let first_publication = relation_owner.computed(source()).expect("a computed source");
    let second_publication = relation_owner.computed(source()).expect("a computed source");
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
    let second = acquire(
        &relation_owner,
        &plan,
        &second_publication,
        ViewContext::default(),
        &mut scratch,
    );
    let distinct_storage = first.storage_identity() != second.storage_identity();
    let one_view_bytes = ByteCount(owner.working_bytes().0 / 2);
    owner.release(first_population);
    let released_after_first = owner.released_entries();
    let one_view_left = owner.working_bytes() == one_view_bytes;
    let again = acquire(
        &relation_owner,
        &plan,
        &second_publication,
        ViewContext::default(),
        &mut scratch,
    );
    let second_reused = second.storage_identity() == again.storage_identity();
    let counts = owner.counts();
    owner.release(second_population);
    OwnViews {
        distinct_storage,
        released_after_first,
        one_view_left,
        second_reused,
        counts,
        working_after_both: owner.working_bytes().0,
        released_after_both: owner.released_entries(),
    }
}

fn shared_view(rounds: u64) -> SharedView {
    let relation_owner = owner(EntryLimit(0));
    let owner = relation_owner.derived();
    let mut ordered_source = source();
    ordered_source.cells = [3, 1, 2]
        .into_iter()
        .map(|value| CellValue::Number(Integer(value)))
        .collect();
    let publications = [
        relation_owner
            .computed(ordered_source.clone())
            .expect("a computed source"),
        relation_owner.computed(ordered_source).expect("a computed source"),
    ];
    let populations = [
        population(&relation_owner, &publications[0]),
        population(&relation_owner, &publications[1]),
    ];
    let plan = pair_plan();
    let mut scratch = DerivedScratch::default();
    let mut cache = indexes::new(&[], &[], ByteLimit(4096)).expect("an empty index cache");
    let metadata_charge = cache.bytes.0;
    cache.limit.0 = 0;
    drop(owner.backend.borrow_mut().replace(cache));
    let mut stable_metadata = None;
    let mut held = SharedView {
        survived_first_release: 0,
        working_after_first_release: 0,
        reused_after_first_repin: 0,
        survived_second_release: 0,
        reused_after_second_release: 0,
        dropped_after_both_released: 0,
        no_working_bytes_after_both: 0,
        metadata_charge_kept: 0,
        one_entry_one_dependent_each: 0,
        metadata_stable: 0,
        builds: BuildCount(0),
    };
    let count = |holds: bool| u64::from(holds);
    for _ in 0..rounds {
        owner.pin(populations[0]);
        owner.pin(populations[1]);
        let first = acquire_pair(&relation_owner, &plan, &publications, &mut scratch);
        let weak = Arc::downgrade(first.storage());
        drop(first);

        owner.release(populations[0]);
        held.survived_first_release += count(weak.upgrade().is_some());
        held.working_after_first_release += count(owner.working_bytes().0 > 0);
        owner.pin(populations[0]);
        let after_first = acquire_pair(&relation_owner, &plan, &publications, &mut scratch);
        held.reused_after_first_repin += count(
            weak.upgrade()
                .is_some_and(|view| Arc::ptr_eq(after_first.storage(), &view)),
        );
        drop(after_first);

        owner.release(populations[1]);
        held.survived_second_release += count(weak.upgrade().is_some());
        let after_second = acquire_pair(&relation_owner, &plan, &publications, &mut scratch);
        held.reused_after_second_release += count(
            weak.upgrade()
                .is_some_and(|view| Arc::ptr_eq(after_second.storage(), &view)),
        );
        drop(after_second);

        owner.release(populations[0]);
        held.dropped_after_both_released += count(weak.upgrade().is_none());
        held.no_working_bytes_after_both += count(owner.working_bytes().0 == 0);
        held.metadata_charge_kept += count(
            owner
                .backend
                .borrow()
                .as_ref()
                .is_some_and(|cache| cache.bytes.0 == metadata_charge),
        );
        let state = owner.state.borrow();
        held.one_entry_one_dependent_each += count(
            state.entries.len() == 1
                && state.dependents[&populations[0]].len() == 1
                && state.dependents[&populations[1]].len() == 1,
        );
        drop(state);
        let bytes = owner.metadata_bytes();
        held.metadata_stable += count(*stable_metadata.get_or_insert(bytes) == bytes);
    }
    held.builds = owner.counts().0;
    held
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
