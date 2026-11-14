//! Shared lookup, lazy physical construction, ownership and retention decision for a population-scoped view.
use super::types::{Acquisition, Construction, OptionalRetention, StoredObservation};
use crate::application::file_dag::accounting::footprint::service::backed_bytes;
use crate::application::file_dag::accounting::ledger::types::ByteCount;
use crate::application::file_dag::derived::constants;
use crate::application::file_dag::derived::owner::acquisition::admission::service::{
    AcquiredView, AcquisitionLifetime, AdmittedViewInputs,
};
use crate::application::file_dag::derived::owner::acquisition::semantics::storage::service::DerivedStorage;
use crate::application::file_dag::derived::owner::cache::types::CacheBackend;
use crate::application::file_dag::derived::owner::graph_key::service::RequirementIdentity;
use crate::application::file_dag::derived::owner::service::*;
use crate::application::file_dag::derived::owner::types::*;
use crate::application::file_dag::derived::owner::*;
use crate::application::file_dag::derived::population::service::PopulationId;
use crate::application::file_dag::route_counts;
use ryo_derived::domain::request::types::DerivedKind;
use ryo_execution_domain::domain::{
    derived_views::types::contract::value::DerivedContract, operating::errors::OperatingError,
};
use std::sync::Arc;

impl DerivedOwner {
    pub(crate) fn acquire_shared_with_requirement(
        &self,
        requirement: RequirementIdentity,
        plan: &Arc<DerivedContract>,
        admitted: &AdmittedViewInputs<'_>,
        context: ViewContext,
        stored_observation: StoredObservation,
        mut cache: Option<&mut CacheBackend>,
        build: impl FnOnce(
            Option<&mut CacheBackend>,
        ) -> Result<Acquisition<Arc<DerivedStorage>>, OperatingError>,
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
    pub(crate) fn try_acquire_shared_with_requirement(
        &self,
        requirement: RequirementIdentity,
        plan: &Arc<DerivedContract>,
        admitted: &AdmittedViewInputs<'_>,
        context: ViewContext,
        stored_observation: StoredObservation,
        mut cache: Option<&mut CacheBackend>,
        build: impl FnOnce(
            Option<&mut CacheBackend>,
        ) -> Result<Option<Acquisition<Arc<DerivedStorage>>>, OperatingError>,
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
        let Ok(len) = u8::try_from(populations.len()) else {
            return Err(OperatingError::ShapeMismatch);
        };
        let key = Key {
            plan: requirement,
            populations: ids,
            len: PopulationCount(len),
            context,
        };
        let _owner = route_counts::service::owner(&self.routes, std::ptr::from_ref(self).addr());
        let _identity = route_counts::service::identity(
            &self.routes,
            Arc::as_ptr(plan).addr(),
            ids.map(|id| (id.0 != usize::MAX).then_some(id.0)),
        );
        route_counts::service::event(
            &self.routes,
            constants::OWNER_REQUEST,
            route_counts::service::derived(plan.kind),
        );
        {
            let mut guard = self.state.borrow_mut();
            // One plain borrow of the state, so the entry found once and the owner's totals are borrowed apart.
            let state = &mut *guard;
            let required = populations
                .iter()
                .any(|id| state.pins.get(id).is_some_and(|n| n.0 > 0));
            if let Some(entry) = state.entries.get_mut(&key)
                && let Some(storage) = entry.weak.upgrade()
            {
                let backings = Arc::clone(&entry.backings);
                let lease = match entry.lease.upgrade() {
                    Some(lease) => lease,
                    None => self.required_lease(&backings)?,
                };
                let handle = self.complete_view(
                    plan,
                    storage,
                    lease,
                    backings,
                    admitted,
                    Construction::Reused,
                )?;
                let promote = required && entry.required.is_none();
                if promote {
                    entry.required = Some(OwnedView {
                        storage: Arc::clone(handle.storage()),
                        _lease: Arc::clone(handle.lease()),
                    });
                    let (bytes, backend, overlap) = (
                        entry.working,
                        entry.backend,
                        if entry.retained.is_some() {
                            entry.bytes
                        } else {
                            ByteCount(0)
                        },
                    );
                    state.required_bytes += bytes;
                    state.required_backend_bytes += backend;
                    state.overlap_bytes += overlap;
                    state.required_views += 1;
                    state.peak_required_views = state.peak_required_views.max(state.required_views);
                    state.peak_required_bytes = state.peak_required_bytes.max(state.required_bytes);
                }
                entry.lease = Arc::downgrade(handle.lease());
                state.hits += 1;
                route_counts::service::event(
                    &self.routes,
                    constants::OWNER_HIT,
                    route_counts::service::derived(plan.kind),
                );
                if let Some(cache) = cache.as_deref_mut() {
                    if stored_observation == StoredObservation::Observed
                        && plan.kind == DerivedKind::Equality
                    {
                        cache.reuses += 1_u64;
                    }
                }
                return Ok(Some(handle));
            }
        }
        let Some(made) = build(cache.as_deref_mut())? else {
            return Ok(None);
        };
        let Acquisition {
            storage,
            construction,
            optional,
        } = made;
        let built = construction == Construction::Built;
        let allow_optional = optional == OptionalRetention::Allowed;
        if stored_observation == StoredObservation::Observed && plan.kind == DerivedKind::Equality {
            if let Some(cache) = cache.as_deref_mut() {
                cache.builds += u64::from(built);
            }
        }
        let _checked = storage.view()?;
        let bytes = backed_bytes(&DerivedStorage::backings(&storage)?)?;
        let backend_bytes = ByteCount(0);
        let working = bytes.saturating_add(backend_bytes);
        let backings = backings::service::view(&storage)?;
        let handle = self.complete_view(
            plan,
            Arc::clone(&storage),
            self.required_lease(&backings)?,
            Arc::clone(&backings),
            admitted,
            construction,
        )?;
        let optional = self.ledger.borrow();
        let additional_bytes = optional.incremental_optional_bytes(&backings)?.0;
        let optional_bytes = optional.snapshot().view_optional_bytes.0;
        drop(optional);
        let additional_positions = self
            .ledger
            .borrow()
            .incremental_optional_units(&backings)?
            .0;
        let positions = self.ledger.borrow().snapshot().optional_units.0;
        let positions_within_bound = positions
            .checked_add(additional_positions)
            .is_some_and(|n| n <= cache::types::MAX_RETAINED_POSITION_CAPACITY);
        let mut state = self.state.borrow_mut();
        state.builds += u64::from(built);
        state.hits += u64::from(!built);
        let required = populations
            .iter()
            .any(|id| state.pins.get(id).is_some_and(|n| n.0 > 0));
        let known = state.entries.contains_key(&key);
        let optional_room = known || state.entries.len() < self.max_entries.0;
        if !required && !optional_room {
            state.refusals += 1_u64;
            return Ok(Some(handle));
        }
        let retained = if let Some(cache) = cache.as_deref_mut().filter(|cache| {
            optional_room
                && allow_optional
                && positions_within_bound
                && optional_bytes
                    .checked_add(additional_bytes)
                    .is_some_and(|bytes| bytes <= cache.limit.0)
                && cache
                    .bytes
                    .0
                    .checked_add(additional_bytes)
                    .is_some_and(|n| n <= cache.limit.0)
        }) {
            let lease = self.optional_lease(&backings)?;
            cache.bytes.0 += additional_bytes;
            cache.graph_counts.peak_bytes.0 = cache.graph_counts.peak_bytes.0.max(cache.bytes.0);
            state.retained_bytes += bytes;
            Some(OwnedView {
                storage: Arc::clone(&storage),
                _lease: lease,
            })
        } else {
            if allow_optional {
                state.refusals += 1_u64;
            }
            None
        };
        if let Some(cache) = cache.as_deref_mut() {
            if stored_observation == StoredObservation::Observed
                && plan.kind == DerivedKind::Equality
                && allow_optional
                && retained.is_none()
            {
                cache.refusals += 1_u64;
            }
        }
        if required {
            if retained.is_some() {
                state.overlap_bytes += bytes;
            }
            state.required_views += 1;
            state.peak_required_views = state.peak_required_views.max(state.required_views);
            state.required_backend_bytes += backend_bytes;
            state.required_bytes += working;
            state.peak_required_bytes = state.peak_required_bytes.max(state.required_bytes);
        }
        let admission = state.next_admission;
        state.next_admission = AdmissionOrder(
            state
                .next_admission
                .0
                .checked_add(1)
                .ok_or(OperatingError::ShapeMismatch)?,
        );
        if !known {
            link(&mut state, key.clone(), populations);
        }
        state.entries.insert(
            key,
            OwnerEntry {
                admission,
                _plan: Arc::clone(plan),
                weak: Arc::downgrade(&storage),
                required: required.then(|| OwnedView {
                    storage: Arc::clone(handle.storage()),
                    _lease: Arc::clone(handle.lease()),
                }),
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
