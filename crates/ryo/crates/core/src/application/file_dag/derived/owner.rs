//! Central acquisition, mandatory working ownership and optional retained capacity.
use super::footprint::storage_bytes;
use crate::application::file_dag::derived::backend::CacheBackend;
use crate::application::file_dag::route_counts::BuildRoute;
use crate::domain::{
    derived_views::{collections::views::registration::registered, types::*},
    operating::errors::OperatingError,
};
use std::{
    cell::RefCell,
    collections::HashMap,
    sync::{Arc, Weak},
};

mod diagnostics;
#[path = "owner/lifetime.rs"]
mod lifetime;
mod observations;
pub(super) mod selection;
pub(in crate::application::file_dag) use observations::IndexObservation;
mod acquisition;
mod backings;
mod graph;
mod leases;
#[cfg(test)]
pub(crate) mod resource_fixture;
mod scoped;
#[cfg(test)]
pub(crate) mod selection_fixture;
mod setup;

/// Assigned only after application admission; canonical DAG producers keep one ID across rules.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(in crate::application::file_dag) struct PopulationId(
    pub(in crate::application::file_dag) usize,
);
/// Runtime semantic dependencies that are not columns: candidate filter and exact dense ordering.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub(in crate::application::file_dag) struct ViewContext {
    pub candidate: Option<usize>,
    pub membership_order: Option<usize>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Key {
    plan: usize,
    populations: [PopulationId; 2],
    len: u8,
    context: ViewContext,
}
struct RetainedView {
    storage: Arc<DerivedStorage>,
    _lease: Arc<super::ledger::ResourceLease>,
}
struct Entry {
    _plan: Arc<DerivedContract>,
    weak: Weak<DerivedStorage>,
    required: Option<super::view::AcquiredView>,
    retained: Option<RetainedView>,
    backings: Arc<[super::ledger::Backing]>,
    lease: Weak<super::ledger::ResourceLease>,
    bytes: usize,
    working: usize,
    backend: usize,
}
#[derive(Default)]
struct State {
    entries: HashMap<Key, Entry>,
    pins: HashMap<PopulationId, usize>,
    dependents: HashMap<PopulationId, Vec<Key>>,
    spare_dependencies: Vec<Vec<Key>>,
    released_entries: u64,
    builds: u64,
    hits: u64,
    refusals: u64,
    required_bytes: usize,
    peak_required_bytes: usize,
    retained_bytes: usize,
    retained_predicates: usize,
    overlap_bytes: usize,
    required_views: usize,
    peak_required_views: usize,
    required_backend_bytes: usize,
}
pub(crate) struct DerivedOwner {
    next_scope: std::cell::Cell<usize>,
    ledger: std::rc::Rc<RefCell<super::ledger::Ledger>>,
    backend: RefCell<Option<CacheBackend>>,
    state: RefCell<State>,
    max_entries: usize,
}
impl DerivedOwner {
    pub(in crate::application::file_dag) fn new(max_entries: usize) -> Self {
        Self {
            next_scope: std::cell::Cell::new(0),
            ledger: std::rc::Rc::new(RefCell::new(super::ledger::Ledger::default())),
            state: RefCell::new(State::default()),
            backend: RefCell::new(None),
            max_entries,
        }
    }
    /// A graph's existing priority can reclaim optional wrappers, but never a
    /// required or borrowed view. Payload retained in CacheBackend remains charged there.
    pub(in crate::application::file_dag) fn reclaim_optional(
        &self,
        cache: &mut CacheBackend,
        cost: usize,
    ) {
        let mut state = self.state.borrow_mut();
        let mut released = 0;
        let mut released_predicates = 0;
        for entry in state.entries.values_mut() {
            if cache
                .bytes
                .saturating_sub(released)
                .checked_add(cost)
                .is_some_and(|bytes| bytes <= cache.limit)
            {
                break;
            }
            if entry.required.is_none()
                && entry
                    .retained
                    .as_ref()
                    .is_some_and(|storage| Arc::strong_count(&storage.storage) == 1)
            {
                let _released = entry.retained.take();
                released_predicates +=
                    usize::from(entry._plan.kind == DerivedKind::PredicateMatches);
                released += entry.bytes;
            }
        }
        state.retained_bytes = state.retained_bytes.saturating_sub(released);
        state.retained_predicates = state
            .retained_predicates
            .saturating_sub(released_predicates);
        cache.bytes = cache.bytes.saturating_sub(released);
    }
    pub(in crate::application::file_dag) fn counts(&self) -> (u64, u64, u64) {
        let state = self.state.borrow();
        (state.builds, state.hits, state.refusals)
    }
    pub(in crate::application::file_dag) fn working_bytes(&self) -> usize {
        self.state.borrow().required_bytes
    }
    pub(in crate::application::file_dag) fn peak_working_bytes(&self) -> usize {
        self.state.borrow().peak_required_bytes
    }
    pub(in crate::application::file_dag) fn retained_bytes(&self) -> usize {
        self.state.borrow().retained_bytes
    }
    pub(in crate::application::file_dag) fn required_backend_overlap(&self) -> usize {
        self.state.borrow().required_backend_bytes
    }
    pub(in crate::application::file_dag) fn peak_required_views(&self) -> usize {
        self.state.borrow().peak_required_views
    }
    pub(in crate::application::file_dag) fn owned_bytes(&self) -> usize {
        let state = self.state.borrow();
        state
            .required_bytes
            .saturating_add(state.retained_bytes)
            .saturating_sub(state.overlap_bytes)
    }
    pub(in crate::application::file_dag) fn metadata_bytes(&self) -> usize {
        let state = self.state.borrow();
        state
            .entries
            .capacity()
            .saturating_mul(size_of::<(Key, Entry)>() + 1)
            .saturating_mul(2)
            + state
                .pins
                .capacity()
                .saturating_mul(size_of::<(PopulationId, usize)>() + 1)
                .saturating_mul(2)
            + state
                .dependents
                .capacity()
                .saturating_mul(size_of::<(PopulationId, Vec<Key>)>() + 1)
                .saturating_mul(2)
            + state
                .dependents
                .values()
                .map(|keys| keys.capacity().saturating_mul(size_of::<Key>()))
                .sum::<usize>()
            + state
                .spare_dependencies
                .capacity()
                .saturating_mul(size_of::<Vec<Key>>())
            + state
                .spare_dependencies
                .iter()
                .map(|keys| keys.capacity().saturating_mul(size_of::<Key>()))
                .sum::<usize>()
    }
}

#[cfg(test)]
impl DerivedOwner {
    pub(in crate::application::file_dag) fn released_entries(&self) -> u64 {
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
            .expect("installed reverse list")
            .push(key);
    }
}

impl Default for DerivedOwner {
    fn default() -> Self {
        Self::new(0)
    }
}
#[cfg(test)]
#[path = "owner_test.rs"]
mod owner_test;

/// Current-partition work is mandatory but deliberately not retained past its cursor window.
/// The application supplies and reuses the complete owned storage; semantics cannot schedule it.
pub(in crate::application::file_dag) fn build_scoped(
    route: BuildRoute,
    plan: &DerivedContract,
    inputs: DerivedInputs<'_, '_>,
    storage: &mut DerivedStorage,
    scratch: &mut DerivedScratch,
) -> Result<(), OperatingError> {
    super::super::route_counts::observe(
        route.label(),
        super::super::route_counts::derived(plan.kind),
        || registered(plan.kind).build(plan, inputs, storage, scratch),
    )?;
    let _ready = registered(plan.kind).view(plan, storage)?;
    Ok(())
}
