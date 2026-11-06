//! Invocation-lifetime builds and acquisitions that no population pin retains.
use crate::application::file_dag::derived::owner::*;
use crate::application::file_dag::derived::ledger::Backing;
use crate::application::file_dag::derived::owner::acquisition::admission::service::{AcquiredView, AcquisitionLifetime, AdmittedViewInputs};

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

    pub(in crate::application::file_dag::derived::owner::acquisition) fn acquire_invocation_with(
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
        let mut required = crate::application::file_dag::derived::owner::backings::view(&storage)?.to_vec();
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
