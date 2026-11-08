//! Central acquisition, mandatory working ownership and optional retained capacity.
use crate::application::file_dag::derived::constants;
use crate::application::file_dag::derived::ledger::types::ByteCount;
use crate::application::file_dag::derived::owner::cache::types::CacheBackend;
use crate::application::file_dag::derived::population::types::PopulationId;
use ryo_execution_domain::domain::{
    derived_views::types::{contract::value::DerivedContract, *},
    operating::errors::OperatingError,
};
use acquisition::semantics::storage::DerivedStorage;
use footprint::service::storage_bytes;
use std::{
    cell::RefCell,
    sync::{Arc, Weak},
};
use rustc_hash::FxHashMap;
use types::{
    AdmissionOrder, BuildCount, BuildCounts, EntryLimit, FilePreparation,
    HitCount, IdentityCompatibility, MembershipOrder, MemoBudget, PinCount, PinState,
    PopulationCount, ReclaimOutlook, RefusalCount, ReleasedEntries, Room, ScopeId,
    ViewCount,
};

pub(crate) mod acquisition;
mod backings;
pub(crate) mod cached_graph;
mod diagnostics;
mod footprint;
mod graph;
mod graph_policy;
mod graph_retention;
mod leases;
mod lifetime;
mod setup;
pub(crate) mod types;

/// Runtime semantic dependencies that are not columns: candidate filter and exact dense ordering.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub(crate) struct ViewContext {
    pub candidate: Option<graph_key::PreparedIdentity>,
    pub literals: Option<Arc<[ryo_values::domain::plan_parts::types::LiteralText]>>,
    pub membership_order: Option<MembershipOrder>,
}
impl ViewContext {
    #[cfg(test)]
    pub(crate) fn for_predicate(plan: &Arc<DerivedContract>) -> Self {
        Self {
            candidate: Some(graph_key::PreparedIdentity::new(plan)),
            ..Self::default()
        }
    }
}
mod graph_key;
use graph_key::{GraphRequirement, GraphViewIdentity, RequirementIdentity};
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Key {
    plan: RequirementIdentity,
    populations: [PopulationId; 2],
    len: PopulationCount,
    context: ViewContext,
}
struct OwnedView {
    storage: Arc<DerivedStorage>,
    _lease: Arc<super::ledger::types::ResourceLease>,
}
struct Entry {
    admission: AdmissionOrder,
    _plan: Arc<DerivedContract>,
    weak: Weak<DerivedStorage>,
    required: Option<OwnedView>,
    retained: Option<OwnedView>,
    backings: Arc<[super::ledger::types::Backing]>,
    lease: Weak<super::ledger::types::ResourceLease>,
    bytes: ByteCount,
    working: ByteCount,
    backend: ByteCount,
}
#[derive(Default)]
struct State {
    next_admission: AdmissionOrder,
    entries: hashbrown::HashMap<Key, Entry>,
    pins: FxHashMap<PopulationId, PinCount>,
    dependents: FxHashMap<PopulationId, Vec<Key>>,
    spare_dependencies: Vec<Vec<Key>>,
    released_entries: ReleasedEntries,
    builds: BuildCount,
    hits: HitCount,
    refusals: RefusalCount,
    required_bytes: ByteCount,
    peak_required_bytes: ByteCount,
    retained_bytes: ByteCount,
    overlap_bytes: ByteCount,
    required_views: ViewCount,
    peak_required_views: ViewCount,
    required_backend_bytes: ByteCount,
}
pub(crate) struct DerivedOwner {
    identity: Arc<()>,
    semantic: types::DerivedAccess,
    next_scope: std::cell::Cell<ScopeId>,
    #[cfg(test)]
    last_scoped_storage: RefCell<Weak<DerivedStorage>>,
    ledger: std::rc::Rc<RefCell<super::ledger::types::Ledger>>,
    backend: RefCell<Option<CacheBackend>>,
    file_prepared: std::cell::Cell<FilePreparation>,
    state: RefCell<State>,
    max_entries: EntryLimit,
    /// The route profile its relation owner hands down.
    routes: crate::application::file_dag::route_counts::types::RouteProfile,
}
impl DerivedOwner {
    #[cfg(test)]
    pub(crate) fn prepare_view(
        &self,
        request: &ViewRequest,
        schemas: &[ryo_execution_domain::domain::operation_schema::types::InputShape<'_>],
    ) -> Result<DerivedContract, OperatingError> {
        super::prepare::view(self.semantic.clone(), request, schemas)
    }

    pub(crate) fn new(
        max_entries: EntryLimit,
        semantic: impl Into<types::DerivedAccess>,
        routes: crate::application::file_dag::route_counts::types::RouteProfile,
    ) -> Self {
        Self {
            identity: Arc::new(()),
            semantic: semantic.into(),
            next_scope: std::cell::Cell::new(ScopeId(0)),
            #[cfg(test)]
            last_scoped_storage: RefCell::new(Weak::new()),
            ledger: std::rc::Rc::new(RefCell::new(super::ledger::types::Ledger::default())),
            state: RefCell::new(State::default()),
            backend: RefCell::new(None),
            file_prepared: std::cell::Cell::new(FilePreparation::Unprepared),
            max_entries,
            routes,
        }
    }
    pub(crate) fn counts(&self) -> BuildCounts {
        let state = self.state.borrow();
        BuildCounts(state.builds, state.hits, state.refusals)
    }
    pub(crate) fn working_bytes(&self) -> ByteCount {
        self.state.borrow().required_bytes
    }
    pub(crate) fn peak_working_bytes(&self) -> ByteCount {
        self.state.borrow().peak_required_bytes
    }
    pub(crate) fn retained_bytes(&self) -> ByteCount {
        self.state.borrow().retained_bytes
    }
    pub(crate) fn required_backend_overlap(&self) -> ByteCount {
        self.state.borrow().required_backend_bytes
    }
    pub(crate) fn peak_required_views(&self) -> ViewCount {
        self.state.borrow().peak_required_views
    }
    pub(crate) fn owned_bytes(&self) -> ByteCount {
        let state = self.state.borrow();
        ByteCount(
            state
                .required_bytes
                .saturating_add(state.retained_bytes)
                .0
                .saturating_sub(state.overlap_bytes.0),
        )
    }
    pub(crate) fn metadata_bytes(&self) -> ByteCount {
        use crate::application::file_dag::derived::ledger::{
            capacity::{saturating, table},
            types::SlotCount,
        };
        let state = self.state.borrow();
        let plans = state
            .entries
            .keys()
            .map(|key| key.plan.metadata_bytes().0)
            .sum::<usize>();
        let spare = state
            .spare_dependencies
            .iter()
            .map(|keys| saturating::<Key>(SlotCount(keys.capacity())).0)
            .sum::<usize>();
        let dependent_keys = state
            .dependents
            .values()
            .map(|keys| saturating::<Key>(SlotCount(keys.capacity())).0)
            .sum::<usize>();
        ByteCount(
            plans
                + table::<(Key, Entry)>(SlotCount(state.entries.capacity()))
                    .0
                    .saturating_mul(2)
                + table::<(PopulationId, PinCount)>(SlotCount(state.pins.capacity()))
                    .0
                    .saturating_mul(2)
                + table::<(PopulationId, Vec<Key>)>(SlotCount(state.dependents.capacity()))
                    .0
                    .saturating_mul(2)
                + dependent_keys
                + saturating::<Vec<Key>>(SlotCount(state.spare_dependencies.capacity())).0
                + spare,
        )
    }
}

#[cfg(test)]
impl DerivedOwner {
    pub(crate) fn released_entries(&self) -> ReleasedEntries {
        self.state.borrow().released_entries
    }
}

fn link(state: &mut State, key: Key, populations: &[PopulationId]) {
    for &population in populations {
        if !state.dependents.contains_key(&population) {
            let spare = state.spare_dependencies.pop().unwrap_or_default();
            state.dependents.insert(population, spare);
        }
        state
            .dependents
            .get_mut(&population)
            .expect(constants::EXPECT_REVERSE_LIST)
            .push(key.clone());
    }
}

#[cfg(test)]
mod test;


use ryo_derived::domain::request::types::DerivedKind;

pub(crate) mod cache;
pub(crate) mod graphs;
pub(crate) mod index;

#[cfg(test)]
pub(crate) mod witness;

#[cfg(test)]
pub(crate) mod fixtures;
