//! Entry points that acquire one population-scoped derived view.
use super::super::*;
use super::admission::service::{AcquiredView, AcquisitionLifetime, AdmittedViewInputs};
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
}
