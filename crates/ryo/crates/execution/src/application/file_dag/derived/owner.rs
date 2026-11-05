//! Central acquisition, mandatory working ownership and optional retained capacity.
use super::footprint::storage_bytes;
use super::storage::DerivedStorage;
use crate::application::file_dag::derived::backend::CacheBackend;
use crate::application::file_dag::route_counts::BuildRoute;
use crate::domain::{derived_views::types::*, operating::errors::OperatingError};
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
#[cfg(test)]
pub(in crate::application::file_dag) use observations::IndexObservation;
mod acquisition;
mod backings;
mod graph;
#[cfg(test)]
pub(crate) mod graph_fixture;
#[cfg(test)]
mod graph_lifetime_test;
mod graph_policy;
mod graph_retention;
#[cfg(test)]
#[path = "owner/index_lifetime_test.rs"]
mod index_lifetime_test;
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
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub(in crate::application::file_dag) struct ViewContext {
    pub candidate: Option<graph_key::PreparedIdentity>,
    pub literals: Option<Arc<[ryo_values::domain::plan_parts::types::LiteralText]>>,
    pub membership_order: Option<usize>,
}
impl ViewContext {
    pub(in crate::application::file_dag) fn for_predicate(plan: &Arc<DerivedContract>) -> Self {
        Self {
            candidate: Some(graph_key::PreparedIdentity::new(plan)),
            ..Self::default()
        }
    }
}
mod graph_key;
pub(in crate::application::file_dag::derived) use graph_key::{
    GraphRequirement, GraphSelection, GraphViewIdentity, RequirementIdentity,
};
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Key {
    plan: RequirementIdentity,
    populations: [PopulationId; 2],
    len: u8,
    context: ViewContext,
}
struct RetainedView {
    storage: Arc<DerivedStorage>,
    _lease: Arc<super::ledger::ResourceLease>,
}
struct Entry {
    admission: usize,
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
    next_admission: usize,
    entries: hashbrown::HashMap<Key, Entry>,
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
    semantic: Arc<dyn crate::outbound::derived::interface::DerivedPort>,
    next_scope: std::cell::Cell<usize>,
    ledger: std::rc::Rc<RefCell<super::ledger::Ledger>>,
    backend: RefCell<Option<CacheBackend>>,
    file_prepared: std::cell::Cell<bool>,
    state: RefCell<State>,
    max_entries: usize,
}
impl DerivedOwner {
    /// A finite semantic workspace belongs to this exact live population owner.
    /// The descriptor was prepared once by the request; undeclared raw orders refuse.
    pub(in crate::application::file_dag) fn finite_raw(
        &self,
        owner: std::rc::Rc<crate::application::file_dag::relations::RelationOwner>,
        prepared: Option<Arc<ryo_derived::domain::contract::types::PreparedDerived>>,
    ) -> Result<impl crate::domain::operator::contract::derived::ScopedDerived, OperatingError>
    {
        if !std::ptr::eq(self, &owner.derived)
            || prepared.as_ref().is_some_and(|plan| {
                !matches!(
                    plan.request(),
                    ryo_derived::domain::contract::types::ViewRequest::RawKeyOrder { .. }
                )
            })
        {
            return Err(OperatingError::WrongOperation);
        }
        Ok(raw_scope::RawScope::for_program(owner, prepared))
    }

    pub(in crate::application::file_dag) fn semantic_port(
        &self,
    ) -> Arc<dyn crate::outbound::derived::interface::DerivedPort> {
        Arc::clone(&self.semantic)
    }

    pub(in crate::application::file_dag) fn new(
        max_entries: usize,
        semantic: Arc<dyn crate::outbound::derived::interface::DerivedPort>,
    ) -> Self {
        Self {
            semantic,
            next_scope: std::cell::Cell::new(0),
            ledger: std::rc::Rc::new(RefCell::new(super::ledger::Ledger::default())),
            state: RefCell::new(State::default()),
            backend: RefCell::new(None),
            file_prepared: std::cell::Cell::new(false),
            max_entries,
        }
    }
    pub(super) fn reclaim_optional_preserving(
        &self,
        cache: &mut CacheBackend,
        cost: usize,
        protected: &[super::ledger::Backing],
        extra_units: usize,
    ) {
        let mut state = self.state.borrow_mut();
        let ledger = self.ledger.borrow();
        let mut candidates = state
            .entries
            .iter()
            .filter_map(|(key, entry)| {
                (entry.required.is_none()
                    && ledger.optional_payload_only(&entry.backings)
                    && entry
                        .retained
                        .as_ref()
                        .is_some_and(|retained| Arc::strong_count(&retained.storage) == 1)
                    && !entry
                        .backings
                        .iter()
                        .any(|backing| protected.iter().any(|needed| needed.key == backing.key)))
                .then(|| (entry.admission, key.clone()))
            })
            .collect::<Vec<_>>();
        drop(ledger);
        candidates.sort_unstable_by_key(|candidate| candidate.0);
        let mut released = 0;
        let mut released_predicates = 0;
        let mut released_indexes = 0;
        for (_, key) in candidates {
            if cache
                .bytes
                .checked_add(cost)
                .is_some_and(|bytes| bytes <= cache.limit)
                && self
                    .ledger
                    .borrow()
                    .snapshot()
                    .optional_units
                    .checked_add(extra_units)
                    .is_some_and(|units| units <= super::selection::MAX_RETAINED_POSITION_CAPACITY)
            {
                break;
            }
            let before = self.ledger.borrow().snapshot().view_optional_bytes;
            let Some(entry) = state.entries.get_mut(&key) else {
                continue;
            };
            let removed = entry.retained.take();
            released += entry.bytes;
            released_predicates += usize::from(entry._plan.kind == DerivedKind::PredicateMatches);
            released_indexes += u64::from(entry._plan.kind == DerivedKind::Equality);
            // Drop the optional lease before observing the physical union. A
            // graph or publication alias may still own some of these backings.
            drop(removed);
            let after = self.ledger.borrow().snapshot().view_optional_bytes;
            let physical = before.saturating_sub(after);
            cache.bytes = cache.bytes.saturating_sub(physical);
            cache.graph_counts.reclaimed_bytes += physical as u64;
        }
        state.retained_bytes = state.retained_bytes.saturating_sub(released);
        state.retained_predicates = state
            .retained_predicates
            .saturating_sub(released_predicates);
        cache.graph_counts.reclaimed_selections += released_predicates as u64;
        cache.graph_counts.reclaimed_indexes += released_indexes;
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
            .keys()
            .map(|key| key.plan.metadata_bytes())
            .sum::<usize>()
            + state
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
            .push(key.clone());
    }
}

#[cfg(test)]
#[path = "owner_test.rs"]
mod owner_test;

pub(in crate::application::file_dag) mod raw_scope;

use ryo_derived::domain::contract::types::DerivedKind;
