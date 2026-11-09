//! Invocation-lifetime builds and acquisitions that no population pin retains.
use crate::application::file_dag::derived::ledger::types::Backing;
use crate::application::file_dag::derived::ledger::types::{
    BackingAddress, BackingKey, BackingTag, ByteCount, UnitCount,
};
use crate::application::file_dag::derived::owner::acquisition::admission::service::{
    AcquiredView, AcquisitionLifetime, AdmittedViewInputs,
};
use crate::application::file_dag::derived::owner::acquisition::population::types::{
    Construction, InvocationStorage,
};
use crate::application::file_dag::derived::owner::*;

impl DerivedOwner {
    pub(crate) fn scoped_build(
        &self,
        route: crate::application::file_dag::route_counts::types::BuildRoute,
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
            &self.routes,
            route.label(),
            crate::application::file_dag::route_counts::derived(plan.kind),
            || self.build_admitted(plan, admitted, inputs, scratch, recycled),
        )?
        .ok_or(OperatingError::WrongOperation)?;
        let _checked = storage.view()?;
        let storage = Arc::new(storage);
        #[cfg(any(test, feature = "fixtures"))]
        self.last_scoped_storage.replace(Arc::downgrade(&storage));
        let mut backings = DerivedStorage::backings(&storage)?.to_vec();
        let id = self.next_scope.get().0;
        self.next_scope.set(
            crate::application::file_dag::derived::owner::types::ScopeId(
                id.checked_add(1).ok_or(OperatingError::ShapeMismatch)?,
            ),
        );
        let semantic_bytes = ryo_derived::domain::footprint::service::working_bytes(&scratch.semantic)
            .map_err(ryo_execution_domain::domain::operating::service::derived_fault)?
            .0;
        let scratch_bytes = crate::application::file_dag::derived::ledger::capacity::checked::<
            ryo_relations::domain::relations::types::Value,
        >(
            crate::application::file_dag::derived::ledger::types::SlotCount(scratch.row.capacity()),
        )
        .and_then(|bytes| bytes.0.checked_add(semantic_bytes))
        .ok_or(OperatingError::ShapeMismatch)?;
        backings.push(Backing {
            key: BackingKey {
                tag: BackingTag(16),
                address: BackingAddress(id),
            },
            bytes: ByteCount(scratch_bytes),
            retention_units: UnitCount(0),
        });
        let backings: Arc<[Backing]> = backings.into();
        self.complete_view(
            plan,
            storage,
            self.required_lease(&backings)?,
            backings,
            admitted,
            Construction::Built,
        )
    }

    pub(crate) fn acquire_invocation_with(
        &self,
        plan: &Arc<DerivedContract>,
        admitted: &AdmittedViewInputs<'_>,
        backings: &[Backing],
        build: impl FnOnce() -> Result<InvocationStorage, OperatingError>,
    ) -> Result<AcquiredView, OperatingError> {
        admitted.validate_origin(self, plan)?;
        if admitted.lifetime() != AcquisitionLifetime::Invocation {
            return Err(OperatingError::WrongOperation);
        }
        let InvocationStorage {
            storage,
            construction,
        } = build()?;
        let mut required =
            crate::application::file_dag::derived::owner::backings::view(&storage)?.to_vec();
        for backing in backings {
            crate::application::file_dag::derived::ledger::lease::append_backing(
                &mut required,
                *backing,
            )?;
        }
        let required: Arc<[Backing]> = required.into();
        self.complete_view(
            plan,
            storage,
            self.required_lease(&required)?,
            required,
            admitted,
            construction,
        )
    }
}
