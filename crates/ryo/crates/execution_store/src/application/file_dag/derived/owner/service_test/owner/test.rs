//! Required sharing and optional retention are independent ownership decisions.
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
use super::super::fixtures::{acquire, owner, plan, population, source};
use crate::cases::{Case, CaseName, Expect, Word, run};

use crate::application::file_dag::derived::owner::index::service as indexes;
use crate::application::file_dag::derived::owner::index::types::ByteLimit;
use ryo_derived::domain::contract::types::DerivedView;

/// How the owner's views of the plan are acquired.
#[derive(Clone, Copy, Debug)]
enum Acquired {
    /// Twice, for one pinned population, with no entry and no retained budget.
    TwicePinned,
    /// Twice, unpinned under a one-entry limit, the first dropped before the second.
    AfterTheFirstExpires,
    /// Once each for this many fresh unpinned publications, under no entry limit.
    OncePerPublication(u64),
    /// For two pinned populations, and for the first again in another membership context.
    PerPopulationAndContext,
}

/// What two acquisitions for one pinned population observed.
#[derive(Debug, PartialEq)]
struct Shared {
    one_storage: bool,
    order: [usize; 3],
    builds: BuildCount,
    hits: HitCount,
    retained_bytes: usize,
    working: bool,
    working_is_peak: bool,
    working_after_release: usize,
    released: u64,
}

/// What the owner held after its acquisitions.
#[derive(Debug, PartialEq)]
enum Observed {
    Shared(Shared),
    /// Whether the first storage was alive before and after the second acquisition, the builds
    /// and hits, and the entries held.
    Replaced {
        alive_before: bool,
        alive_after: bool,
        builds: BuildCount,
        hits: HitCount,
        entries: usize,
    },
    /// The entries and dependents held, whether the identity metadata grew, and the counts.
    Missed {
        entries: usize,
        dependents: usize,
        metadata_grew: bool,
        counts: BuildCounts,
    },
    /// Whether the views of the two populations and of the other context are three distinct
    /// storages, and the counts.
    Distinct {
        distinct: [bool; 3],
        counts: BuildCounts,
    },
}

type Input = Acquired;
type Value = Observed;
type Failure = String;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("pinned_population_shares_even_with_zero_entry_and_retained_budget"),
        word: Word::Succeeds,
        input: || Acquired::TwicePinned,
        expect: Expect::Value(Observed::Shared(Shared {
            one_storage: true,
            order: [1, 0, 2],
            builds: BuildCount(1),
            hits: HitCount(1),
            retained_bytes: 0,
            working: true,
            working_is_peak: true,
            working_after_release: 0,
            released: 1,
        })),
    },
    Case {
        name: CaseName("expired_weak_entry_is_replaced_without_retaining_stale_storage"),
        word: Word::Succeeds,
        input: || Acquired::AfterTheFirstExpires,
        expect: Expect::Value(Observed::Replaced {
            alive_before: false,
            alive_after: false,
            builds: BuildCount(2),
            hits: HitCount(0),
            entries: 1,
        }),
    },
    Case {
        name: CaseName("optional_misses_do_not_accumulate_identity_metadata"),
        word: Word::Succeeds,
        input: || Acquired::OncePerPublication(256),
        expect: Expect::Value(Observed::Missed {
            entries: 0,
            dependents: 0,
            metadata_grew: false,
            counts: BuildCounts(BuildCount(256), HitCount(0), RefusalCount(256)),
        }),
    },
    Case {
        name: CaseName("population_and_semantic_context_are_part_of_the_view_identity"),
        word: Word::Succeeds,
        input: || Acquired::PerPopulationAndContext,
        expect: Expect::Value(Observed::Distinct {
            distinct: [true, true, true],
            counts: BuildCounts(BuildCount(3), HitCount(0), RefusalCount(3)),
        }),
    },
];

/// Acquires the partition-order view of the source `3, 1, 3` as the input says.
fn subject(input: Input) -> Result<Value, Failure> {
    let plan = plan();
    let mut scratch = DerivedScratch::default();
    let computed = |relation_owner: &crate::application::file_dag::relations::RelationOwner| {
        relation_owner
            .computed(source())
            .map_err(|error| format!("{error:?}"))
    };
    Ok(match input {
        Acquired::TwicePinned => {
            let relation_owner = owner(EntryLimit(0));
            let owner = relation_owner.derived();
            let publication = computed(&relation_owner)?;
            let population = population(&relation_owner, &publication);
            owner.pin(population);
            let mut cache = indexes::new(&[], &[], ByteLimit(4096)).ok_or("no index cache")?;
            cache.limit.0 = 0;
            drop(owner.backend.borrow_mut().replace(cache));
            let first = acquire(&relation_owner, &plan, &publication, ViewContext::default(), &mut scratch);
            let second = acquire(&relation_owner, &plan, &publication, ViewContext::default(), &mut scratch);
            let DerivedView::PartitionOrder(rows) = first.view().map_err(|error| format!("{error:?}"))? else {
                return Err("not a prepared ordering view".to_owned());
            };
            let mut order = [usize::MAX; 3];
            for (at, row) in order.iter_mut().zip(rows.iter()) {
                *at = row.0;
            }
            let one_storage = first.storage_identity() == second.storage_identity();
            let working = owner.working_bytes();
            let shared = Shared {
                one_storage,
                order,
                builds: owner.counts().0,
                hits: owner.counts().1,
                retained_bytes: owner.retained_bytes().0,
                working: working.0 > 0,
                working_is_peak: working == owner.peak_working_bytes(),
                working_after_release: 0,
                released: 0,
            };
            owner.release(population);
            Observed::Shared(Shared {
                working_after_release: owner.working_bytes().0,
                released: owner.released_entries().0,
                ..shared
            })
        }
        Acquired::AfterTheFirstExpires => {
            let relation_owner = owner(EntryLimit(1));
            let owner = relation_owner.derived();
            let publication = computed(&relation_owner)?;
            let first = acquire(&relation_owner, &plan, &publication, ViewContext::default(), &mut scratch);
            let weak = Arc::downgrade(first.storage());
            drop(first);
            let alive_before = weak.upgrade().is_some();
            let second = acquire(&relation_owner, &plan, &publication, ViewContext::default(), &mut scratch);
            let replaced = Observed::Replaced {
                alive_before,
                alive_after: weak.upgrade().is_some(),
                builds: owner.counts().0,
                hits: owner.counts().1,
                entries: owner.state.borrow().entries.len(),
            };
            drop(second);
            replaced
        }
        Acquired::OncePerPublication(publications) => {
            let relation_owner = owner(EntryLimit(0));
            let owner = relation_owner.derived();
            let initial_metadata = owner.metadata_bytes();
            for _ in 0..publications {
                let publication = computed(&relation_owner)?;
                drop(acquire(&relation_owner, &plan, &publication, ViewContext::default(), &mut scratch));
            }
            let state = owner.state.borrow();
            Observed::Missed {
                entries: state.entries.len(),
                dependents: state.dependents.len(),
                metadata_grew: owner.metadata_bytes() != initial_metadata,
                counts: owner.counts(),
            }
        }
        Acquired::PerPopulationAndContext => {
            let relation_owner = owner(EntryLimit(0));
            let owner = relation_owner.derived();
            let first_publication = computed(&relation_owner)?;
            let second_publication = computed(&relation_owner)?;
            owner.pin(population(&relation_owner, &first_publication));
            owner.pin(population(&relation_owner, &second_publication));
            let first = acquire(&relation_owner, &plan, &first_publication, ViewContext::default(), &mut scratch);
            let other_population = acquire(&relation_owner, &plan, &second_publication, ViewContext::default(), &mut scratch);
            let other_context = acquire(
                &relation_owner,
                &plan,
                &first_publication,
                ViewContext {
                    membership_order: Some(MembershipOrder(2)),
                    ..ViewContext::for_predicate(&plan)
                },
                &mut scratch,
            );
            Observed::Distinct {
                distinct: [
                    first.storage_identity() != other_population.storage_identity(),
                    first.storage_identity() != other_context.storage_identity(),
                    other_population.storage_identity() != other_context.storage_identity(),
                ],
                counts: owner.counts(),
            }
        }
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
