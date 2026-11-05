//! Required current-partition storage is retained only by its actual invocation consumers.
use super::super::ledger::Backing;
use super::*;
impl DerivedOwner {
    pub(in crate::application::file_dag) fn scoped_build(
        &self,
        route: crate::application::file_dag::route_counts::BuildRoute,
        plan: &Arc<DerivedContract>,
        inputs: DerivedInputs<'_, '_>,
        recycled: Option<super::super::view::AcquiredView>,
        scratch: &mut DerivedScratch,
    ) -> Result<super::super::view::AcquiredView, OperatingError> {
        let recycled = recycled.and_then(super::super::view::AcquiredView::into_storage);
        let storage = crate::application::file_dag::route_counts::observe(
            route.label(),
            crate::application::file_dag::route_counts::derived(plan.kind),
            || {
                super::super::semantics::build(
                    self.semantic_port(),
                    plan,
                    inputs,
                    scratch,
                    recycled,
                )
            },
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
        Ok(super::super::view::AcquiredView::new(
            plan,
            storage,
            self.required_lease(&backings)?,
            backings,
            true,
        ))
    }
}
