//! Declared current-partition work is owned until its actual cursor/consumer releases it.
use super::super::ledger::{Backing, ResourceLease};
use super::*;
impl DerivedOwner {
    pub(in crate::application::file_dag) fn scoped_build(
        &self,
        route: crate::application::file_dag::route_counts::BuildRoute,
        plan: &DerivedContract,
        inputs: DerivedInputs<'_, '_>,
        storage: &mut DerivedStorage,
        scratch: &mut DerivedScratch,
    ) -> Result<Arc<ResourceLease>, OperatingError> {
        crate::application::file_dag::route_counts::observe(
            route.label(),
            crate::application::file_dag::route_counts::derived(plan.kind),
            || registered(plan.kind).build(plan, inputs, storage, scratch),
        )?;
        let _checked = registered(plan.kind).view(plan, storage)?;
        let id = self.next_scope.get();
        self.next_scope
            .set(id.checked_add(1).ok_or(OperatingError::ShapeMismatch)?);
        let bytes = storage_bytes(storage)?
            .checked_add(
                scratch
                    .row
                    .capacity()
                    .saturating_mul(size_of::<ryo_language::domain::relations::types::Value>()),
            )
            .ok_or(OperatingError::ShapeMismatch)?;
        self.required_lease(&[Backing {
            key: (16, id),
            bytes,
            retention_units: 0,
        }])
    }
}
