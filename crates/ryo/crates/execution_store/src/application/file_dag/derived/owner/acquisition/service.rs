//! Entry points that acquire one population-scoped derived view.
use super::super::*;
use super::admission::service::{AcquiredView, AcquisitionLifetime, AdmittedViewInputs};
use super::population::types::{
    Acquisition, Construction, OptionalRetention, StoredObservation,
};
use super::population::types::StoredColumns;
use crate::application::file_dag::route_counts::{self, types::BuildRoute};

impl DerivedOwner {
    pub(in crate::application::file_dag) fn acquire_stored(
        &self,
        plan: &Arc<DerivedContract>,
        admitted: &AdmittedViewInputs<'_>,
        context: ViewContext,
        inputs: DerivedInputs<'_, '_>,
        scratch: &mut DerivedScratch,
        stored: Option<StoredColumns>,
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
        stored: Option<StoredColumns>,
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
        stored: Option<StoredColumns>,
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
        stored: Option<StoredColumns>,
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
            if stored.is_some() {
                StoredObservation::Observed
            } else {
                StoredObservation::Unobserved
            },
            cache,
            |cache| {
                let allow_optional = match (stored, cache.as_deref()) {
                    (Some(stored), Some(cache)) if plan.kind == DerivedKind::Equality => {
                        cache.eligible.contains(&stored.relation)
                    }
                    (Some(_), None) if plan.kind == DerivedKind::Equality => false,
                    _ => true,
                };
                let storage =
                    route_counts::observe(&self.routes, route.label(), route_counts::derived(plan.kind), || {
                        self.build_admitted(plan, admitted, inputs, scratch, None)
                    })?;
                // Optional retention is decided below by this owner, never by the kernel.
                Ok(storage.map(|storage| Acquisition {
                    storage: Arc::new(storage),
                    construction: Construction::Built,
                    optional: if allow_optional {
                        OptionalRetention::Allowed
                    } else {
                        OptionalRetention::Refused
                    },
                }))
            },
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
        crate::application::file_dag::derived::owner::acquisition::semantics::service::build(
            Arc::clone(self.semantic.port()),
            plan,
            inputs,
            scratch,
            recycled,
        )
    }
}
