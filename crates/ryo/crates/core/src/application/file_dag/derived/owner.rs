//! Central acquisition, mandatory working ownership and optional retained capacity.
use super::footprint::storage_bytes;
use crate::domain::{
    derived_views::{collections::views::registration::registered, types::*},
    operating::errors::OperatingError,
    stored_indexes::types::FileIndexes,
};
use std::{
    cell::RefCell,
    collections::HashMap,
    sync::{Arc, Weak},
};

#[path = "owner/lifetime.rs"]
mod lifetime;

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
struct Entry {
    _plan: Arc<DerivedContract>,
    weak: Weak<DerivedStorage>,
    required: Option<Arc<DerivedStorage>>,
    retained: Option<Arc<DerivedStorage>>,
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
    overlap_bytes: usize,
    required_views: usize,
    peak_required_views: usize,
    required_backend_bytes: usize,
}
pub(crate) struct DerivedOwner {
    state: RefCell<State>,
    max_entries: usize,
}
impl DerivedOwner {
    pub(in crate::application::file_dag) fn new(max_entries: usize) -> Self {
        Self {
            state: RefCell::new(State::default()),
            max_entries,
        }
    }
    #[cfg(test)]
    pub(in crate::application::file_dag) fn acquire(
        &self,
        plan: &Arc<DerivedContract>,
        populations: &[PopulationId],
        identity: ViewContext,
        inputs: DerivedInputs<'_, '_>,
        scratch: &mut DerivedScratch,
        cache: Option<&mut FileIndexes>,
    ) -> Result<Arc<DerivedStorage>, OperatingError> {
        self.acquire_stored(plan, populations, identity, inputs, scratch, None, cache)
    }
    pub(in crate::application::file_dag) fn acquire_stored(
        &self,
        plan: &Arc<DerivedContract>,
        populations: &[PopulationId],
        context: ViewContext,
        inputs: DerivedInputs<'_, '_>,
        scratch: &mut DerivedScratch,
        stored: Option<(
            ryo_language::domain::relations::types::Relation,
            ryo_language::domain::relations::types::ColumnsView<'_>,
        )>,
        mut cache: Option<&mut FileIndexes>,
    ) -> Result<Arc<DerivedStorage>, OperatingError> {
        if populations.is_empty() || populations.len() > 2 {
            return Err(OperatingError::ShapeMismatch);
        }
        let mut ids = [PopulationId(usize::MAX); 2];
        ids[..populations.len()].copy_from_slice(populations);
        let key = Key {
            plan: Arc::as_ptr(plan) as usize,
            populations: ids,
            len: populations.len() as u8,
            context,
        };
        {
            let mut state = self.state.borrow_mut();
            let required = populations
                .iter()
                .any(|id| state.pins.get(id).is_some_and(|count| *count > 0));
            if let Some(storage) = state
                .entries
                .get(&key)
                .and_then(|entry| entry.weak.upgrade())
            {
                let promote = required
                    && state
                        .entries
                        .get(&key)
                        .is_some_and(|entry| entry.required.is_none());
                if promote {
                    let entry = state.entries.get_mut(&key).expect("existing hit");
                    entry.required = Some(Arc::clone(&storage));
                    let bytes = entry.working;
                    let backend = entry.backend;
                    let overlap = usize::from(entry.retained.is_some()) * entry.bytes;
                    state.required_backend_bytes += backend;
                    state.overlap_bytes += overlap;
                    state.required_views += 1;
                    state.peak_required_views = state.peak_required_views.max(state.required_views);
                    state.required_bytes += bytes;
                    state.peak_required_bytes = state.peak_required_bytes.max(state.required_bytes);
                }
                state.hits += 1;
                if stored.is_some() && plan.kind == DerivedKind::Equality {
                    if let Some(cache) = cache.as_deref_mut() {
                        cache.reuses = cache.reuses.saturating_add(1);
                    }
                }
                return Ok(storage);
            }
        }
        let mut storage = DerivedStorage::Unbuilt;
        let mut built = true;
        let mut backend = false;
        if let (DerivedKind::Equality, Some((relation, source)), Some(cache)) =
            (plan.kind, stored, cache.as_deref_mut())
        {
            let ViewRequest::Equality { columns } = &plan.request else {
                return Err(OperatingError::WrongOperation);
            };
            let mut index = crate::domain::table_model::types::KeyIndex::default();
            let acquired =
                super::indexes::acquire(plan, cache, relation, source, columns, &mut index)?;
            built = acquired.built;
            backend = true;
            storage = acquired.shared.map_or(
                DerivedStorage::Equality(index),
                DerivedStorage::EqualityShared,
            );
        } else {
            registered(plan.kind).build(plan, inputs, &mut storage, scratch)?;
        }
        // A family implementation cannot publish an unbuilt or different-kind object.
        let _checked = registered(plan.kind).view(plan, &storage)?;
        let bytes = storage_bytes(&storage)?;
        let backend_bytes = if let DerivedStorage::EqualityShared(index) = &storage {
            crate::domain::stored_indexes::service::index_bytes(index)
                .ok_or(OperatingError::ShapeMismatch)?
        } else {
            0
        };
        let working = bytes.saturating_add(backend_bytes);
        let storage = Arc::new(storage);
        let mut state = self.state.borrow_mut();
        state.builds += u64::from(built);
        state.hits += u64::from(!built);
        let required = populations
            .iter()
            .any(|id| state.pins.get(id).is_some_and(|count| *count > 0));
        let known = state.entries.contains_key(&key);
        let optional_room = known || state.entries.len() < self.max_entries;
        if !required && !optional_room {
            state.refusals += 1;
            return Ok(storage);
        }
        let retained = cache.as_deref_mut().filter(|cache| {
            optional_room
                && (!backend || matches!(storage.as_ref(), DerivedStorage::EqualityShared(_)))
                && cache
                    .bytes
                    .checked_add(bytes)
                    .is_some_and(|after| after <= cache.limit)
        });
        let retained = if let Some(cache) = retained {
            cache.bytes += bytes;
            cache.graph_counts.peak_bytes = cache.graph_counts.peak_bytes.max(cache.bytes);
            state.retained_bytes += bytes;
            Some(Arc::clone(&storage))
        } else {
            state.refusals += 1;
            None
        };
        if required {
            state.overlap_bytes += usize::from(retained.is_some()) * bytes;
            state.required_views += 1;
            state.peak_required_views = state.peak_required_views.max(state.required_views);
            state.required_backend_bytes += backend_bytes;
            state.required_bytes += working;
            state.peak_required_bytes = state.peak_required_bytes.max(state.required_bytes);
        }
        if !known {
            link(&mut state, key, populations);
        }
        state.entries.insert(
            key,
            Entry {
                _plan: Arc::clone(plan),
                weak: Arc::downgrade(&storage),
                required: required.then(|| Arc::clone(&storage)),
                retained,
                bytes,
                working,
                backend: backend_bytes,
            },
        );
        Ok(storage)
    }
    /// A graph's existing priority can reclaim optional wrappers, but never a
    /// required or borrowed view. Payload retained in FileIndexes remains charged there.
    pub(in crate::application::file_dag) fn reclaim_optional(
        &self,
        cache: &mut FileIndexes,
        cost: usize,
    ) {
        let mut state = self.state.borrow_mut();
        let mut released = 0;
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
                    .is_some_and(|storage| Arc::strong_count(storage) == 1)
            {
                entry.retained.take();
                released += entry.bytes;
            }
        }
        state.retained_bytes = state.retained_bytes.saturating_sub(released);
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
    plan: &DerivedContract,
    inputs: DerivedInputs<'_, '_>,
    storage: &mut DerivedStorage,
    scratch: &mut DerivedScratch,
) -> Result<(), OperatingError> {
    registered(plan.kind).build(plan, inputs, storage, scratch)?;
    let _ready = registered(plan.kind).view(plan, storage)?;
    Ok(())
}
