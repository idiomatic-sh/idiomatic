//! Owner-bound selector acquisition; immutable views leave this scope, never the backend.
use super::*;
use crate::domain::{operating::types::OperatorContext, stored_selection::types::Selector};
use ryo_language::domain::relations::types::ColumnsView;
#[derive(Clone, Copy)]
pub(in crate::application::file_dag::derived) struct SelectionSource<'a> {
    owner: &'a DerivedOwner,
    population: PopulationId,
}
impl SelectionSource<'_> {
    pub(in crate::application::file_dag::derived) fn acquire_predicate(
        &self,
        cache: &mut CacheBackend,
        plan: &Arc<DerivedContract>,
        group: usize,
        build: impl FnOnce(&mut CacheBackend) -> Result<DerivedStorage, OperatingError>,
    ) -> Result<super::super::view::AcquiredView, OperatingError> {
        let repeated = *cache
            .selector_repetition
            .get(group)
            .ok_or(OperatingError::NotRun)?;
        let optional = !cache.selector_repetition_failed && repeated >= 2;
        self.owner.acquire_with(
            plan,
            &[self.population],
            ViewContext::default(),
            false,
            Some(cache),
            |cache| {
                let cache = cache.ok_or(OperatingError::NotRun)?;
                Ok((build(cache)?, true, false, optional))
            },
        )
    }
    pub(in crate::application::file_dag::derived) fn acquire(
        &self,
        route: crate::application::file_dag::route_counts::BuildRoute,
        cache: &mut CacheBackend,
        plan: &Arc<DerivedContract>,
        context: ViewContext,
        inputs: DerivedInputs<'_, '_>,
        scratch: &mut DerivedScratch,
        stored: Option<(
            ryo_language::domain::relations::types::Relation,
            ColumnsView<'_>,
        )>,
    ) -> Result<super::super::view::AcquiredView, OperatingError> {
        self.owner.acquire_in(
            route,
            plan,
            &[self.population],
            context,
            inputs,
            scratch,
            stored,
            Some(cache),
        )
    }
}
impl DerivedOwner {
    pub(in crate::application::file_dag) fn selector_plan(
        &self,
        program: &Arc<Selector>,
    ) -> Result<Option<Arc<DerivedContract>>, OperatingError> {
        let backend = self.backend.borrow();
        let Some(backend) = backend.as_ref() else {
            return Ok(None);
        };
        let Some(plans) = backend.selection_views.as_ref() else {
            return Ok(None);
        };
        let Some(views) = plans.by_selector.get(&(Arc::as_ptr(program) as usize)) else {
            return Ok(None);
        };
        if backend.selector_repetition_failed {
            return Ok(None);
        }
        if backend.selector_repetition.get(views.group).is_none() {
            return Err(OperatingError::NotRun);
        }
        Ok(Some(Arc::clone(&views.predicate)))
    }
    pub(in crate::application::file_dag) fn select<'f>(
        &self,
        program: &Arc<Selector>,
        source: ColumnsView<'f>,
        population: PopulationId,
        context: &mut OperatorContext<'_>,
    ) -> Result<Option<super::super::selection::SelectionOutput<'f>>, OperatingError> {
        let mut backend = self.backend.borrow_mut();
        let Some(backend) = backend.as_mut() else {
            return Ok(None);
        };
        super::super::selection::select(
            backend,
            SelectionSource {
                owner: self,
                population,
            },
            program,
            source,
            context,
        )
    }
}
