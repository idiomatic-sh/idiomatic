//! Owner-bound selector acquisition; immutable views leave this scope, never the backend.
use super::*;
use crate::domain::{operating::types::OperatorContext, stored_selection::types::Selector};
use ryo_relations::domain::relations::types::ColumnsView;
#[derive(Clone, Copy)]
pub(in crate::application::file_dag::derived) struct SelectionSource<'a> {
    owner: &'a DerivedOwner,
    population: PopulationId,
}
impl SelectionSource<'_> {
    pub(in crate::application::file_dag::derived) fn build(
        &self,
        plan: &DerivedContract,
        inputs: DerivedInputs<'_, '_>,
        scratch: &mut DerivedScratch,
    ) -> Result<DerivedStorage, OperatingError> {
        super::super::semantics::build(self.owner.semantic_port(), plan, inputs, scratch, None)?
            .ok_or(OperatingError::WrongOperation)
    }
    pub(in crate::application::file_dag::derived) fn acquire_postings(
        &self,
        cache: &mut CacheBackend,
        plan: &Arc<DerivedContract>,
        source: ColumnsView<'_>,
        texts: &crate::domain::table_texts::types::Texts<'_>,
        active: Arc<[ryo_values::domain::plan_parts::types::LiteralText]>,
    ) -> Result<Option<super::super::view::AcquiredView>, OperatingError> {
        let context = ViewContext {
            literals: Some(Arc::clone(&active)),
            ..ViewContext::default()
        };
        self.owner.try_acquire_shared_with_requirement(
            RequirementIdentity::prepared(plan),
            plan,
            &[self.population],
            context,
            false,
            Some(cache),
            |cache| {
                let cache = cache.ok_or(OperatingError::NotRun)?;
                let state = self.owner.state.borrow();
                let mut groups = 0_usize;
                let mut literals = active.len();
                let mut text_bytes = active
                    .iter()
                    .try_fold(0_usize, |total, value| total.checked_add(value.0.len()))
                    .ok_or(OperatingError::ShapeMismatch)?;
                for entry in state
                    .entries
                    .values()
                    .filter(|entry| entry._plan.kind == DerivedKind::CandidatePostings)
                {
                    let Some(retained) = &entry.retained else {
                        continue;
                    };
                    let ryo_derived::domain::contract::types::DerivedView::Postings(postings) =
                        retained.storage.view()?
                    else {
                        return Err(OperatingError::WrongOperation);
                    };
                    groups += 1;
                    literals = literals
                        .checked_add(postings.len())
                        .ok_or(OperatingError::ShapeMismatch)?;
                    text_bytes = postings
                        .iter()
                        .try_fold(text_bytes, |total, value| {
                            total.checked_add(value.text_capacity())
                        })
                        .ok_or(OperatingError::ShapeMismatch)?;
                }
                drop(state);
                let population = source.columns.first().map_or(0, |column| column.0.len());
                let units = self.owner.ledger.borrow().snapshot().optional_units;
                if groups >= super::super::selection::MAX_FANOUT_GROUPS
                    || literals > super::super::selection::MAX_FANOUT_LITERALS
                    || text_bytes > super::super::selection::MAX_FANOUT_TEXT_BYTES
                    || population
                        .checked_mul(2)
                        .and_then(|need| units.checked_add(need))
                        .is_none_or(|units| {
                            units > super::super::selection::MAX_RETAINED_POSITION_CAPACITY
                        })
                {
                    return Ok(None);
                }
                let storage = crate::application::file_dag::route_counts::observe(
                    crate::application::file_dag::route_counts::BuildRoute::SelectorPostings
                        .label(),
                    crate::application::file_dag::route_counts::derived(plan.kind),
                    || {
                        self.build(
                            plan,
                            DerivedInputs::Postings {
                                rows:
                                    crate::domain::operator::contract::views::BorrowedRows::Stored(
                                        source,
                                    ),
                                texts,
                                active: &active,
                            },
                            &mut DerivedScratch::default(),
                        )
                    },
                )?;
                Ok(Some((Arc::new(storage), true, false, true)))
            },
        )
    }

    pub(in crate::application::file_dag::derived) fn can_retain_equality(
        &self,
        cache: &CacheBackend,
        relation: ryo_relations::domain::relations::types::Relation,
        source: ColumnsView<'_>,
        columns: &[ryo_values::domain::plan_parts::types::ColumnIndex],
    ) -> bool {
        let live_hint = self.owner.has_live_equality(source, columns);
        let allowed =
            live_hint || super::super::indexes::can_retain(cache, relation, source, columns);
        if !allowed {
            if tracing::enabled!(target: "ryo::diagnostic::selection_detail", tracing::Level::TRACE)
            {
                self.owner.report_selector_occupancy(self.population);
            }
            tracing::trace!(target: "ryo::diagnostic::selection_detail", event = "selector_equality_admission_refused",
                population = ?self.population, relation = ?relation, columns = ?columns,
                rows = source.columns.first().map_or(0, |column| column.0.len()),
                cache_bytes = cache.bytes, cache_limit = cache.limit,
                minimum_index_bytes = super::super::indexes::minimum_bytes(source.columns.first().map_or(0, |column| column.0.len()), columns.len()),
                live_hint, ledger = ?self.owner.ledger.borrow().snapshot());
        }
        allowed
    }

    pub(in crate::application::file_dag::derived) fn acquire_predicate(
        &self,
        cache: &mut CacheBackend,
        plan: &Arc<DerivedContract>,
        group: usize,
        build: impl FnOnce(&mut CacheBackend) -> Result<DerivedStorage, OperatingError>,
    ) -> Result<super::super::view::AcquiredView, OperatingError> {
        if cache
            .selection_views
            .as_ref()
            .is_none_or(|views| group >= views.groups)
        {
            return Err(OperatingError::NotRun);
        }
        let repeated = cache.selector_repetition.get(&group).copied().unwrap_or(0);
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
            ryo_relations::domain::relations::types::Relation,
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
        let backend = backend.as_ref().ok_or(OperatingError::NotRun)?;
        let plans = backend
            .selection_views
            .as_ref()
            .ok_or(OperatingError::NotRun)?;
        let views = plans
            .by_selector
            .get(&(Arc::as_ptr(program) as usize))
            .ok_or(OperatingError::NotRun)?;
        if backend.selector_repetition_failed {
            return Ok(None);
        }
        if views.group >= plans.groups {
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
