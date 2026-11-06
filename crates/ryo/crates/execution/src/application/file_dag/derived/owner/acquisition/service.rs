//! Shared lookup, lazy physical construction, ownership and admission.
use super::super::*;
use super::admission::service::{AcquiredView, AcquisitionLifetime, AdmittedViewInputs};
use crate::application::file_dag::derived::ledger::{Backing, ResourceLease};
use crate::application::file_dag::derived::owner::{index::service as indexes, selection};
use crate::application::file_dag::route_counts::{self, BuildRoute};

impl DerivedOwner {
    pub(in crate::application::file_dag) fn acquire_stored(
        &self,
        plan: &Arc<DerivedContract>,
        admitted: &AdmittedViewInputs<'_>,
        context: ViewContext,
        inputs: DerivedInputs<'_, '_>,
        scratch: &mut DerivedScratch,
        stored: Option<(
            ryo_relations::domain::relations::types::Relation,
            ryo_relations::domain::relations::types::ColumnsView<'_>,
        )>,
    ) -> Result<AcquiredView, OperatingError> {
        self.acquire_stored_with_route(
            BuildRoute::OwnerAcquire,
            plan,
            admitted,
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
        admitted: &AdmittedViewInputs<'_>,
        context: ViewContext,
        inputs: DerivedInputs<'_, '_>,
        scratch: &mut DerivedScratch,
        stored: Option<(
            ryo_relations::domain::relations::types::Relation,
            ryo_relations::domain::relations::types::ColumnsView<'_>,
        )>,
    ) -> Result<AcquiredView, OperatingError> {
        let mut backend = self.backend.borrow_mut();
        self.acquire_in(
            route,
            plan,
            admitted,
            context,
            inputs,
            scratch,
            stored,
            backend.as_mut(),
        )
    }
    pub(in crate::application::file_dag) fn try_acquire_rows(
        &self,
        plan: &Arc<DerivedContract>,
        admitted: &AdmittedViewInputs<'_>,
        inputs: DerivedInputs<'_, '_>,
        scratch: &mut DerivedScratch,
    ) -> Result<Option<AcquiredView>, OperatingError> {
        let mut backend = self.backend.borrow_mut();
        self.try_acquire_in(
            BuildRoute::OwnerAcquire,
            plan,
            admitted,
            ViewContext::default(),
            inputs,
            scratch,
            None,
            backend.as_mut(),
        )
    }
    pub(super) fn acquire_in(
        &self,
        route: BuildRoute,
        plan: &Arc<DerivedContract>,
        admitted: &AdmittedViewInputs<'_>,
        context: ViewContext,
        inputs: DerivedInputs<'_, '_>,
        scratch: &mut DerivedScratch,
        stored: Option<(
            ryo_relations::domain::relations::types::Relation,
            ryo_relations::domain::relations::types::ColumnsView<'_>,
        )>,
        cache: Option<&mut CacheBackend>,
    ) -> Result<AcquiredView, OperatingError> {
        self.try_acquire_in(
            route, plan, admitted, context, inputs, scratch, stored, cache,
        )?
        .ok_or(OperatingError::WrongOperation)
    }
    pub(super) fn try_acquire_in(
        &self,
        route: BuildRoute,
        plan: &Arc<DerivedContract>,
        admitted: &AdmittedViewInputs<'_>,
        context: ViewContext,
        inputs: DerivedInputs<'_, '_>,
        scratch: &mut DerivedScratch,
        stored: Option<(
            ryo_relations::domain::relations::types::Relation,
            ryo_relations::domain::relations::types::ColumnsView<'_>,
        )>,
        cache: Option<&mut CacheBackend>,
    ) -> Result<Option<AcquiredView>, OperatingError> {
        admitted.validate(self, plan, &inputs)?;
        if admitted.lifetime() != AcquisitionLifetime::Population {
            return Err(OperatingError::WrongOperation);
        }
        self.try_acquire_shared_with_requirement(
            RequirementIdentity::prepared(plan),
            plan,
            admitted,
            context,
            stored.is_some(),
            cache,
            |cache| {
                let allow_optional = match (stored, cache.as_deref()) {
                    (Some((relation, _)), Some(cache)) if plan.kind == DerivedKind::Equality => {
                        cache.eligible.contains(&relation)
                    }
                    (Some(_), None) if plan.kind == DerivedKind::Equality => false,
                    _ => true,
                };
                let storage =
                    route_counts::observe(route.label(), route_counts::derived(plan.kind), || {
                        self.build_admitted(plan, admitted, inputs, scratch, None)
                    })?;
                // Optional retention is decided below by this owner, never by the kernel.
                Ok(storage.map(|storage| (Arc::new(storage), true, false, allow_optional)))
            },
        )
    }
    pub(super) fn acquire_with(
        &self,
        plan: &Arc<DerivedContract>,
        admitted: &AdmittedViewInputs<'_>,
        context: ViewContext,
        stored_observation: bool,
        mut cache: Option<&mut CacheBackend>,
        build: impl FnOnce(
            Option<&mut CacheBackend>,
        ) -> Result<(DerivedStorage, bool, bool, bool), OperatingError>,
    ) -> Result<AcquiredView, OperatingError> {
        self.acquire_with_requirement(
            RequirementIdentity::prepared(plan),
            plan,
            admitted,
            context,
            stored_observation,
            cache,
            build,
        )
    }
    pub(super) fn acquire_with_requirement(
        &self,
        requirement: RequirementIdentity,
        plan: &Arc<DerivedContract>,
        admitted: &AdmittedViewInputs<'_>,
        context: ViewContext,
        stored_observation: bool,
        mut cache: Option<&mut CacheBackend>,
        build: impl FnOnce(
            Option<&mut CacheBackend>,
        ) -> Result<(DerivedStorage, bool, bool, bool), OperatingError>,
    ) -> Result<AcquiredView, OperatingError> {
        self.acquire_shared_with_requirement(
            requirement,
            plan,
            admitted,
            context,
            stored_observation,
            cache,
            |cache| {
                build(cache).map(|(storage, built, backend, optional)| {
                    (Arc::new(storage), built, backend, optional)
                })
            },
        )
    }
    pub(super) fn acquire_shared_with_requirement(
        &self,
        requirement: RequirementIdentity,
        plan: &Arc<DerivedContract>,
        admitted: &AdmittedViewInputs<'_>,
        context: ViewContext,
        stored_observation: bool,
        mut cache: Option<&mut CacheBackend>,
        build: impl FnOnce(
            Option<&mut CacheBackend>,
        ) -> Result<(Arc<DerivedStorage>, bool, bool, bool), OperatingError>,
    ) -> Result<AcquiredView, OperatingError> {
        self.try_acquire_shared_with_requirement(
            requirement,
            plan,
            admitted,
            context,
            stored_observation,
            cache,
            |cache| build(cache).map(Some),
        )?
        .ok_or(OperatingError::WrongOperation)
    }
    pub(super) fn try_acquire_shared_with_requirement(
        &self,
        requirement: RequirementIdentity,
        plan: &Arc<DerivedContract>,
        admitted: &AdmittedViewInputs<'_>,
        context: ViewContext,
        stored_observation: bool,
        mut cache: Option<&mut CacheBackend>,
        build: impl FnOnce(
            Option<&mut CacheBackend>,
        )
            -> Result<Option<(Arc<DerivedStorage>, bool, bool, bool)>, OperatingError>,
    ) -> Result<Option<AcquiredView>, OperatingError> {
        admitted.validate_origin(self, plan)?;
        if admitted.lifetime() != AcquisitionLifetime::Population {
            return Err(OperatingError::WrongOperation);
        }
        let populations = admitted.populations();
        if populations.is_empty() || populations.len() > 2 {
            return Err(OperatingError::ShapeMismatch);
        }
        let mut ids = [PopulationId(usize::MAX); 2];
        ids[..populations.len()].copy_from_slice(populations);
        let key = Key {
            plan: requirement,
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
                let handle = self.complete_view(plan, storage, lease, backings, admitted, false)?;
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
                    Arc::downgrade(handle.lease());
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
                return Ok(Some(handle));
            }
        }
        let Some((storage, built, backend, allow_optional)) = build(cache.as_deref_mut())? else {
            return Ok(None);
        };
        if stored_observation && plan.kind == DerivedKind::Equality {
            if let Some(cache) = cache.as_deref_mut() {
                cache.builds = cache.builds.saturating_add(u64::from(built));
            }
        }
        let _checked = storage.view()?;
        let bytes = storage_bytes(&storage)?;
        let backend_bytes = 0;
        let working = bytes.saturating_add(backend_bytes);
        let backings = backings::view(&storage)?;
        let handle = self.complete_view(
            plan,
            Arc::clone(&storage),
            self.required_lease(&backings)?,
            Arc::clone(&backings),
            admitted,
            built,
        )?;
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
            return Ok(Some(handle));
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
                && cache
                    .bytes
                    .checked_add(additional_bytes)
                    .is_some_and(|n| n <= cache.limit)
        }) {
            let lease = self.optional_lease(&backings)?;
            cache.bytes += additional_bytes;
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
            if stored_observation
                && plan.kind == DerivedKind::Equality
                && allow_optional
                && retained.is_none()
            {
                cache.refusals = cache.refusals.saturating_add(1);
            }
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
        let admission = state.next_admission;
        state.next_admission = state
            .next_admission
            .checked_add(1)
            .ok_or(OperatingError::ShapeMismatch)?;
        if !known {
            link(&mut state, key.clone(), populations);
        }
        state.entries.insert(
            key,
            Entry {
                admission,
                _plan: Arc::clone(plan),
                weak: Arc::downgrade(&storage),
                required: required.then(|| handle.clone()),
                retained,
                backings,
                lease: Arc::downgrade(handle.lease()),
                bytes,
                working,
                backend: backend_bytes,
            },
        );
        Ok(Some(handle))
    }
}

impl DerivedOwner {
    pub(in crate::application::file_dag) fn scoped_build(
        &self,
        route: crate::application::file_dag::route_counts::BuildRoute,
        plan: &Arc<DerivedContract>,
        admitted: &AdmittedViewInputs<'_>,
        inputs: DerivedInputs<'_, '_>,
        recycled: Option<AcquiredView>,
        scratch: &mut DerivedScratch,
    ) -> Result<AcquiredView, OperatingError> {
        admitted.validate(self, plan, &inputs)?;
        if admitted.lifetime() != AcquisitionLifetime::Invocation {
            return Err(OperatingError::WrongOperation);
        }
        let recycled = recycled.and_then(AcquiredView::into_storage);
        let storage = crate::application::file_dag::route_counts::observe(
            route.label(),
            crate::application::file_dag::route_counts::derived(plan.kind),
            || self.build_admitted(plan, admitted, inputs, scratch, recycled),
        )?
        .ok_or(OperatingError::WrongOperation)?;
        let _checked = storage.view()?;
        let storage = Arc::new(storage);
        #[cfg(test)]
        self.last_scoped_storage.replace(Arc::downgrade(&storage));
        let mut backings = DerivedStorage::backings(&storage)?.to_vec();
        let id = self.next_scope.get();
        self.next_scope
            .set(id.checked_add(1).ok_or(OperatingError::ShapeMismatch)?);
        let semantic_bytes = scratch
            .semantic
            .working_bytes()
            .map_err(crate::domain::operator::contract::derived::portable_fault)?;
        let scratch_bytes = scratch
            .row
            .capacity()
            .checked_mul(size_of::<ryo_relations::domain::relations::types::Value>())
            .and_then(|bytes| bytes.checked_add(semantic_bytes))
            .ok_or(OperatingError::ShapeMismatch)?;
        backings.push(Backing {
            key: (16, id),
            bytes: scratch_bytes,
            retention_units: 0,
        });
        let backings: Arc<[Backing]> = backings.into();
        self.complete_view(
            plan,
            storage,
            self.required_lease(&backings)?,
            backings,
            admitted,
            true,
        )
    }
}

impl DerivedOwner {
    pub(in crate::application::file_dag::derived::owner::acquisition) fn build_admitted(
        &self,
        plan: &Arc<DerivedContract>,
        admitted: &AdmittedViewInputs<'_>,
        inputs: DerivedInputs<'_, '_>,
        scratch: &mut DerivedScratch,
        recycled: Option<DerivedStorage>,
    ) -> Result<Option<DerivedStorage>, OperatingError> {
        admitted.validate(self, plan, &inputs)?;
        super::semantics::service::build(
            Arc::clone(&self.semantic),
            plan,
            inputs,
            scratch,
            recycled,
        )
    }
}

impl DerivedOwner {
    pub(super) fn acquire_invocation_with(
        &self,
        plan: &Arc<DerivedContract>,
        admitted: &AdmittedViewInputs<'_>,
        backings: &[Backing],
        build: impl FnOnce() -> Result<(Arc<DerivedStorage>, bool), OperatingError>,
    ) -> Result<AcquiredView, OperatingError> {
        admitted.validate_origin(self, plan)?;
        if admitted.lifetime() != AcquisitionLifetime::Invocation {
            return Err(OperatingError::WrongOperation);
        }
        let (storage, built) = build()?;
        let mut required = super::super::backings::view(&storage)?.to_vec();
        for backing in backings {
            crate::application::file_dag::derived::ledger::append_backing(&mut required, *backing)?;
        }
        let required: Arc<[Backing]> = required.into();
        self.complete_view(
            plan,
            storage,
            self.required_lease(&required)?,
            required,
            admitted,
            built,
        )
    }
}
