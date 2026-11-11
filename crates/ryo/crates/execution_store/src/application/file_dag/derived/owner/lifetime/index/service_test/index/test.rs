//! Actual shared-index acquisition, owner aliases, eviction and final consumer release.
use super::fixtures::{fixture, hold, reclaim};
use crate::application::file_dag::derived::owner::*;
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
use crate::application::file_dag::derived::owner::service::*;
use crate::application::file_dag::derived::owner::types::*;
use crate::cases::{Case, CaseName, Expect, Word, run};
use fixtures::resources::ResourceFixture;
use ryo_derived::domain::contract::types::DerivedView;
use ryo_relations::domain::relations::types::FactColumn;

/// What the owner does while a consumer holds the shared equality index over the rows
/// `(1, 10), (2, 20)`.
#[derive(Clone, Copy, Debug)]
enum WhileHeld {
    /// The owner reclaims room, before and after the consumer releases.
    Reclaims,
    /// The owner releases its population, then the consumer releases.
    ReleasesPopulation,
}

/// What reclaiming observed: the room answered and the index's state around each attempt.
#[derive(Debug, PartialEq)]
struct Reclaimed {
    invocation_handles_while_held: bool,
    room_while_held: Room,
    backings_while_held: usize,
    invocation_handles_released: bool,
    alive_before_reclaim: bool,
    room_after_release: Room,
    alive_after_reclaim: bool,
    backings_after_reclaim: usize,
    optional_bytes_after_reclaim: usize,
}

/// What releasing the population observed: the index kept for its consumer, then freed.
#[derive(Debug, PartialEq)]
struct Released {
    alive_after_population: bool,
    backings_after_population: usize,
    optional_bytes_after_population: usize,
    rows_still_read: usize,
    alive_after_consumer: bool,
    backings_after_consumer: usize,
    unique_bytes_after_consumer: usize,
}

/// What the owner and the index observed.
#[derive(Debug, PartialEq)]
enum Observed {
    Reclaimed(Reclaimed),
    Released(Released),
}

type Input = WhileHeld;
type Value = Observed;
type Failure = String;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("one_owner_reclaims_index_only_after_the_final_live_consumer_releases"),
        word: Word::Succeeds,
        input: || WhileHeld::Reclaims,
        expect: Expect::Value(Observed::Reclaimed(Reclaimed {
            invocation_handles_while_held: true,
            room_while_held: Room::Full,
            backings_while_held: 1,
            invocation_handles_released: true,
            alive_before_reclaim: true,
            room_after_release: Room::Fits,
            alive_after_reclaim: false,
            backings_after_reclaim: 0,
            optional_bytes_after_reclaim: 0,
        })),
    },
    Case {
        name: CaseName("backend_clear_preserves_one_required_backing_until_last_consumer_drops"),
        word: Word::Succeeds,
        input: || WhileHeld::ReleasesPopulation,
        expect: Expect::Value(Observed::Released(Released {
            alive_after_population: true,
            backings_after_population: 1,
            optional_bytes_after_population: 0,
            rows_still_read: 2,
            alive_after_consumer: false,
            backings_after_consumer: 0,
            unique_bytes_after_consumer: 0,
        })),
    },
];

fn ledger(fixture: &ResourceFixture) -> crate::application::file_dag::derived::ledger::types::LedgerSnapshot {
    fixture.owner().derived().ledger.borrow().snapshot()
}

/// Holds the shared index for one consumer and lets the owner act while it is held.
fn subject(input: Input) -> Result<Value, Failure> {
    let columns = [FactColumn(vec![1, 2]), FactColumn(vec![10, 20])];
    let fixture = fixture(&columns);
    let weak = fixture.first_index_weak().ok_or("no shared index")?;
    let held = hold(&fixture, &columns);
    Ok(match input {
        WhileHeld::Reclaims => {
            let invocation_handles_while_held = !fixture.no_invocation_index_handles();
            let room_while_held = reclaim(&fixture);
            let backings_while_held = fixture.shared_index_backings();
            drop(held);
            let invocation_handles_released = fixture.no_invocation_index_handles();
            let alive_before_reclaim = weak.upgrade().is_some();
            let room_after_release = reclaim(&fixture);
            Observed::Reclaimed(Reclaimed {
                invocation_handles_while_held,
                room_while_held,
                backings_while_held,
                invocation_handles_released,
                alive_before_reclaim,
                room_after_release,
                alive_after_reclaim: weak.upgrade().is_some(),
                backings_after_reclaim: fixture.shared_index_backings(),
                optional_bytes_after_reclaim: ledger(&fixture).view_optional_bytes.0,
            })
        }
        WhileHeld::ReleasesPopulation => {
            fixture.owner().release_population();
            let alive_after_population = weak.upgrade().is_some();
            let backings_after_population = fixture.shared_index_backings();
            let optional_bytes_after_population = ledger(&fixture).view_optional_bytes.0;
            let DerivedView::Equality(index) = held.view().map_err(|error| format!("{error:?}"))?
            else {
                return Err("the held view is not an equality index".to_owned());
            };
            let rows_still_read = index.rows.len();
            drop(held);
            Observed::Released(Released {
                alive_after_population,
                backings_after_population,
                optional_bytes_after_population,
                rows_still_read,
                alive_after_consumer: weak.upgrade().is_some(),
                backings_after_consumer: fixture.shared_index_backings(),
                unique_bytes_after_consumer: ledger(&fixture).unique_bytes.0,
            })
        }
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
