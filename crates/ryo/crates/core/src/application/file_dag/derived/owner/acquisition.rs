//! Shared lookup, lazy physical construction, ownership and admission.
use super::*;
use crate::application::file_dag::derived::{indexes, selection, view::AcquiredView};
use crate::application::file_dag::route_counts::{self, BuildRoute};
impl DerivedOwner {
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
    ) -> Result<AcquiredView, OperatingError> {
        self.acquire_stored_with_route(
            BuildRoute::OwnerAcquire,
            plan,
            populations,
            context,
            inputs,
            scratch,
            stored,
        )
    }
    pub(in crate::application::file_dag) fn acquire_stored_with_route(
        &self,
        route: BuildRoute,
        plan: &Arc<DerivedContract>,
        populations: &[PopulationId],
        context: ViewContext,
        inputs: DerivedInputs<'_, '_>,
        scratch: &mut DerivedScratch,
        stored: Option<(
            ryo_language::domain::relations::types::Relation,
            ryo_language::domain::relations::types::ColumnsView<'_>,
        )>,
    ) -> Result<AcquiredView, OperatingError> {
        let mut backend = self.backend.borrow_mut();
        self.acquire_in(
            route,
            plan,
            populations,
            context,
            inputs,
            scratch,
            stored,
            backend.as_mut(),
        )
    }
    pub(super) fn acquire_in(
        &self,
        route: BuildRoute,
        plan: &Arc<DerivedContract>,
        populations: &[PopulationId],
        context: ViewContext,
        inputs: DerivedInputs<'_, '_>,
        scratch: &mut DerivedScratch,
        stored: Option<(
            ryo_language::domain::relations::types::Relation,
            ryo_language::domain::relations::types::ColumnsView<'_>,
        )>,
        cache: Option<&mut CacheBackend>,
    ) -> Result<AcquiredView, OperatingError> {
        self.acquire_with(
            plan,
            populations,
            context,
            stored.is_some(),
            cache,
            |mut cache| {
                let mut storage = DerivedStorage::Unbuilt;
                let (built, backend) =
                    if let (DerivedKind::Equality, Some((relation, source)), Some(cache)) =
                        (plan.kind, stored, cache.as_deref_mut())
                    {
                        let ViewRequest::Equality { columns } = &plan.request else {
                            return Err(OperatingError::WrongOperation);
                        };
                        let mut index = crate::domain::table_model::types::KeyIndex::default();
                        let acquired = indexes::acquire(
                            route, plan, cache, relation, source, columns, &mut index,
                        )?;
                        if let Some(slot) = acquired.slot {
                            let entry = cache.entries.get(slot).ok_or(OperatingError::NotRun)?;
                            if entry.payload_lease.is_none() {
                                let backing = backings::shared_index(&entry.index)?;
                                let ledger = self.ledger.borrow();
                                let allowed = ledger.snapshot().view_optional_bytes
                                    .checked_add(ledger.incremental_optional_bytes(&[backing])?)
                                    .is_some_and(|bytes| bytes <= cache.limit);
                                drop(ledger);
                                if allowed {
                                    cache.entries[slot].payload_lease = Some(self.optional_lease(&[backing])?);
                                } else {
                                    let removed = cache.entries.remove(slot);
                                    let cost = crate::application::file_dag::derived::graph_cache::types::reclamation::value::index_payload(&removed)
                                        .ok_or(OperatingError::ShapeMismatch)?;
                                    cache.bytes = cache.bytes.checked_sub(cost).ok_or(OperatingError::ShapeMismatch)?;
                                    cache.refusals += 1;
                                    drop(removed);
                                }
                            }
                        }
                        storage = acquired.shared.map_or(
                            DerivedStorage::Equality(index),
                            DerivedStorage::EqualityShared,
                        );
                        (acquired.built, true)
                    } else {
                        route_counts::observe(
                            route.label(),
                            route_counts::derived(plan.kind),
                            || registered(plan.kind).build(plan, inputs, &mut storage, scratch),
                        )?;
                        (true, false)
                    };
                Ok((storage, built, backend, true))
            },
        )
    }
    pub(super) fn acquire_with(
        &self,
        plan: &Arc<DerivedContract>,
        populations: &[PopulationId],
        context: ViewContext,
        stored_observation: bool,
        mut cache: Option<&mut CacheBackend>,
        build: impl FnOnce(
            Option<&mut CacheBackend>,
        ) -> Result<(DerivedStorage, bool, bool, bool), OperatingError>,
    ) -> Result<AcquiredView, OperatingError> {
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
        let _owner = route_counts::owner(self as *const _ as usize);
        let _identity = route_counts::identity(
            Arc::as_ptr(plan) as usize,
            ids.map(|id| (id.0 != usize::MAX).then_some(id.0)),
        );
        route_counts::event("owner_request", route_counts::derived(plan.kind));
        {
            let mut state = self.state.borrow_mut();
            let required = populations
                .iter()
                .any(|id| state.pins.get(id).is_some_and(|n| *n > 0));
            if let Some(storage) = state
                .entries
                .get(&key)
                .and_then(|entry| entry.weak.upgrade())
            {
                let entry = state.entries.get(&key).expect("existing view");
                let backings = Arc::clone(&entry.backings);
                let lease = match entry.lease.upgrade() {
                    Some(lease) => lease,
                    None => self.required_lease(&backings)?,
                };
                let handle = AcquiredView::new(plan, storage, lease, backings, false);
                let promote = required && entry.required.is_none();
                if promote {
                    let entry = state.entries.get_mut(&key).expect("existing view");
                    entry.required = Some(handle.clone());
                    let (bytes, backend, overlap) = (
                        entry.working,
                        entry.backend,
                        usize::from(entry.retained.is_some()) * entry.bytes,
                    );
                    state.required_bytes += bytes;
                    state.required_backend_bytes += backend;
                    state.overlap_bytes += overlap;
                    state.required_views += 1;
                    state.peak_required_views = state.peak_required_views.max(state.required_views);
                    state.peak_required_bytes = state.peak_required_bytes.max(state.required_bytes);
                }
                state.entries.get_mut(&key).expect("existing view").lease =
                    Arc::downgrade(&handle.lease);
                state.hits += 1;
                route_counts::event("owner_hit", route_counts::derived(plan.kind));
                if let Some(cache) = cache.as_deref_mut() {
                    if stored_observation && plan.kind == DerivedKind::Equality {
                        cache.reuses = cache.reuses.saturating_add(1);
                    }
                    if plan.kind == DerivedKind::PredicateMatches {
                        cache.selection_counts.reuses += 1;
                    }
                }
                return Ok(handle);
            }
        }
        let (storage, built, backend, allow_optional) = build(cache.as_deref_mut())?;
        let _checked = registered(plan.kind).view(plan, &storage)?;
        let bytes = storage_bytes(&storage)?;
        let backend_bytes = if let DerivedStorage::EqualityShared(index) = &storage {
            indexes::index_bytes(index).ok_or(OperatingError::ShapeMismatch)?
        } else {
            0
        };
        let working = bytes.saturating_add(backend_bytes);
        let storage = Arc::new(storage);
        let backings = backings::view(&storage)?;
        let handle = AcquiredView::new(
            plan,
            Arc::clone(&storage),
            self.required_lease(&backings)?,
            Arc::clone(&backings),
            built,
        );
        let optional = self.ledger.borrow();
        let additional_bytes = optional.incremental_optional_bytes(&backings)?;
        let optional_bytes = optional.snapshot().view_optional_bytes;
        drop(optional);
        let additional_positions = self.ledger.borrow().incremental_optional_units(&backings)?;
        let positions = self.ledger.borrow().snapshot().optional_units;
        let positions_within_bound = positions
            .checked_add(additional_positions)
            .is_some_and(|n| n <= selection::MAX_RETAINED_POSITION_CAPACITY);
        let mut state = self.state.borrow_mut();
        state.builds += u64::from(built);
        state.hits += u64::from(!built);
        let required = populations
            .iter()
            .any(|id| state.pins.get(id).is_some_and(|n| *n > 0));
        let known = state.entries.contains_key(&key);
        let optional_room = known || state.entries.len() < self.max_entries;
        if !required && !optional_room {
            state.refusals += 1;
            return Ok(handle);
        }
        let predicate_room = plan.kind != DerivedKind::PredicateMatches
            || state.retained_predicates < selection::MAX_RETAINED_SELECTIONS;
        let retained = if let Some(cache) = cache.as_deref_mut().filter(|cache| {
            optional_room
                && predicate_room
                && allow_optional
                && positions_within_bound
                && optional_bytes
                    .checked_add(additional_bytes)
                    .is_some_and(|bytes| bytes <= cache.limit)
                && (!backend || matches!(storage.as_ref(), DerivedStorage::EqualityShared(_)))
                && cache
                    .bytes
                    .checked_add(bytes)
                    .is_some_and(|n| n <= cache.limit)
        }) {
            let lease = self.optional_lease(&backings)?;
            cache.bytes += bytes;
            cache.graph_counts.peak_bytes = cache.graph_counts.peak_bytes.max(cache.bytes);
            state.retained_bytes += bytes;
            state.retained_predicates += usize::from(plan.kind == DerivedKind::PredicateMatches);
            Some(RetainedView {
                storage: Arc::clone(&storage),
                _lease: lease,
            })
        } else {
            if allow_optional {
                state.refusals += 1;
            }
            None
        };
        if let Some(cache) = cache.as_deref_mut() {
            if plan.kind == DerivedKind::PredicateMatches {
                cache.selection_counts.builds += u64::from(built);
                cache.selection_counts.reuses += u64::from(!built);
                cache.selection_counts.refusals += u64::from(allow_optional && retained.is_none());
            }
        }
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
                required: required.then(|| handle.clone()),
                retained,
                backings,
                lease: Arc::downgrade(&handle.lease),
                bytes,
                working,
                backend: backend_bytes,
            },
        );
        Ok(handle)
    }
}
