//! Owner-bound selector acquisition; immutable views leave this scope, never the backend.
use super::super::super::*;
use super::types::{RowBound, SelectorGroup, StoredColumns};
use crate::application::file_dag::derived::owner::acquisition::population::types::{
    Acquisition, BackendRetention, Construction, OptionalRetention, StoredObservation,
};
use crate::application::file_dag::derived::owner::index::types::{IndexRowCount, KeyColumns};
use crate::application::file_dag::derived::owner::selection::types::{
    Retainability, SelectedPositions,
};
use crate::application::file_dag::relations::{RelationOwner, admission::service::BorrowedInput};
use crate::domain::stored_selection::types::{Selector, SelectorHistogram};
use ryo_derived::domain::contract::types::DerivedView;
use ryo_relations::domain::relations::types::ColumnsView;
#[derive(Clone, Copy)]
pub(in crate::application::file_dag::derived::owner) struct SelectionSource<'a> {
    owner: &'a DerivedOwner,
    source: &'a RelationOwner,
    proof: &'a BorrowedInput<'a>,
    population: PopulationId,
}
impl<'a> SelectionSource<'a> {
    pub(in crate::application::file_dag::derived::owner) fn admit(
        owner: &'a DerivedOwner,
        relation_owner: &'a RelationOwner,
        publication: &crate::application::file_dag::relations::handle::service::PublishedInput,
        proof: &'a BorrowedInput<'a>,
        source: ColumnsView<'_>,
    ) -> Result<Self, OperatingError> {
        proof.require_publication(relation_owner, publication.as_ref())?;
        if !std::ptr::eq(owner, relation_owner.derived())
            || crate::application::file_dag::relations::same_admitted_rows(
                proof.rows(),
                crate::domain::operator::contract::views::BorrowedRows::Stored(source),
            ) == crate::application::file_dag::relations::types::Sameness::Different
        {
            return Err(OperatingError::NotRead);
        }
        let population = proof.identity(relation_owner)?.population;
        Ok(Self {
            owner,
            source: relation_owner,
            proof,
            population,
        })
    }
}
impl SelectionSource<'_> {
    pub(in crate::application::file_dag::derived::owner::acquisition::selection) fn build(
        &self,
        plan: &Arc<DerivedContract>,
        inputs: DerivedInputs<'_, '_>,
        scratch: &mut DerivedScratch,
    ) -> Result<DerivedStorage, OperatingError> {
        let admitted = self.owner.admit_view(plan, self.source, &[self.proof])?;
        self.owner
            .build_admitted(plan, &admitted, inputs, scratch, None)?
            .ok_or(OperatingError::WrongOperation)
    }
    pub(in crate::application::file_dag::derived::owner) fn acquire_postings(
        &self,
        cache: &mut CacheBackend,
        plan: &Arc<DerivedContract>,
        source: ColumnsView<'_>,
        texts: &crate::domain::table_texts::types::Texts<'_>,
        active: Arc<[ryo_values::domain::plan_parts::types::LiteralText]>,
    ) -> Result<
        Option<crate::application::file_dag::derived::owner::acquisition::admission::service::AcquiredView>,
        OperatingError,
    >{
        let context = ViewContext {
            literals: Some(Arc::clone(&active)),
            ..ViewContext::default()
        };
        let admitted = self.owner.admit_view(plan, self.source, &[self.proof])?;
        self.owner.try_acquire_shared_with_requirement(
            RequirementIdentity::prepared(plan),
            plan,
            &admitted,
            context,
            StoredObservation::Unobserved,
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
                let units = self.owner.ledger.borrow().snapshot().optional_units.0;
                if groups >= crate::application::file_dag::derived::owner::selection::types::MAX_FANOUT_GROUPS
                    || literals > crate::application::file_dag::derived::owner::selection::types::MAX_FANOUT_LITERALS
                    || text_bytes > crate::application::file_dag::derived::owner::selection::types::MAX_FANOUT_TEXT_BYTES
                    || population
                        .checked_mul(2)
                        .and_then(|need| units.checked_add(need))
                        .is_none_or(|units| {
                            units > crate::application::file_dag::derived::owner::selection::types::MAX_RETAINED_POSITION_CAPACITY
                        })
                {
                    return Ok(None);
                }
                let storage = crate::application::file_dag::route_counts::observe(
                    crate::application::file_dag::route_counts::types::BuildRoute::SelectorPostings
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
                Ok(Some(Acquisition {
                    storage: Arc::new(storage),
                    construction: Construction::Built,
                    backend: BackendRetention::Unaccounted,
                    optional: OptionalRetention::Allowed,
                }))
            },
        )
    }

    pub(in crate::application::file_dag::derived::owner) fn can_retain_equality(
        &self,
        cache: &CacheBackend,
        relation: ryo_relations::domain::relations::types::Relation,
        source: ColumnsView<'_>,
        columns: &[ryo_values::domain::plan_parts::types::ColumnIndex],
    ) -> Retainability {
        let live_hint = self.owner.has_live_equality(source, columns);
        let allowed = if live_hint == EqualityLiveness::Live {
            Retainability::Retainable
        } else {
            crate::application::file_dag::derived::owner::index::service::can_retain(
                cache, relation, source, columns,
            )
        };
        if allowed == Retainability::Unretainable {
            if tracing::enabled!(target: constants::TARGET_SELECTION_DETAIL, tracing::Level::TRACE)
            {
                self.owner.report_selector_occupancy(self.population);
            }
            tracing::trace!(target: constants::TARGET_SELECTION_DETAIL, event = constants::EVENT_SELECTOR_EQUALITY_REFUSED,
                population = ?self.population, relation = ?relation, columns = ?columns,
                rows = source.columns.first().map_or(0, |column| column.0.len()),
                cache_bytes = cache.bytes.0, cache_limit = cache.limit.0,
                minimum_index_bytes = crate::application::file_dag::derived::owner::index::service::minimum_bytes(IndexRowCount(source.columns.first().map_or(0, |column| column.0.len())), KeyColumns(columns.len())).map(|bytes| bytes.0),
                live_hint = ?live_hint, ledger = ?self.owner.ledger.borrow().snapshot());
        }
        allowed
    }

    pub(in crate::application::file_dag::derived::owner::acquisition::selection) fn acquire_predicate(
        &self,
        cache: &mut CacheBackend,
        plan: &Arc<DerivedContract>,
        group: SelectorGroup,
        build: impl FnOnce(&mut CacheBackend) -> Result<DerivedStorage, OperatingError>,
    ) -> Result<
        crate::application::file_dag::derived::owner::acquisition::admission::service::AcquiredView,
        OperatingError,
    > {
        if cache
            .selection_views
            .as_ref()
            .is_none_or(|views| group.0 >= views.groups)
        {
            return Err(OperatingError::NotRun);
        }
        let repeated = cache
            .selector_repetition
            .get(
                &crate::application::file_dag::derived::owner::cache::types::RepetitionGroup(
                    group.0,
                ),
            )
            .copied()
            .unwrap_or_default();
        let optional = cache.selector_repetition_failed
            == crate::application::file_dag::derived::owner::cache::types::RepetitionTracking::Tracked
            && repeated.0 >= 2;
        let admitted = self.owner.admit_view(plan, self.source, &[self.proof])?;
        self.owner.acquire_with(
            plan,
            &admitted,
            ViewContext::default(),
            StoredObservation::Unobserved,
            Some(cache),
            |cache| {
                let cache = cache.ok_or(OperatingError::NotRun)?;
                Ok(Acquisition {
                    storage: build(cache)?,
                    construction: Construction::Built,
                    backend: BackendRetention::Unaccounted,
                    optional: if optional {
                        OptionalRetention::Allowed
                    } else {
                        OptionalRetention::Refused
                    },
                })
            },
        )
    }
    pub(in crate::application::file_dag::derived::owner) fn acquire(
        &self,
        route: crate::application::file_dag::route_counts::types::BuildRoute,
        cache: &mut CacheBackend,
        plan: &Arc<DerivedContract>,
        context: ViewContext,
        inputs: DerivedInputs<'_, '_>,
        scratch: &mut DerivedScratch,
        stored: Option<StoredColumns>,
    ) -> Result<
        crate::application::file_dag::derived::owner::acquisition::admission::service::AcquiredView,
        OperatingError,
    > {
        let admitted = self.owner.admit_view(plan, self.source, &[self.proof])?;
        self.owner.acquire_in(
            route,
            plan,
            &admitted,
            context,
            inputs,
            scratch,
            stored,
            Some(cache),
        )
    }
}

pub(in crate::application::file_dag::derived::owner) fn histogram<'a>(
    cache: &'a mut CacheBackend,
    program: &Arc<Selector>,
) -> Option<&'a mut SelectorHistogram> {
    let entries = cache.selection_histogram.as_mut()?;
    let at = entries
        .iter()
        .position(|entry| Arc::ptr_eq(&entry.selector, program));
    let at = at.unwrap_or_else(|| {
        entries.push(SelectorHistogram::new(program));
        entries.len() - 1
    });
    entries.get_mut(at)
}

pub(in crate::application::file_dag::derived::owner) fn shared_positions(
    acquired: &crate::application::file_dag::derived::owner::acquisition::admission::service::AcquiredView,
    population: RowBound,
    maximum: RowBound,
) -> Result<SelectedPositions, OperatingError> {
    let DerivedView::RowIds(rows) = acquired.view()? else {
        return Err(OperatingError::WrongOperation);
    };
    if rows.len() > maximum.0
        || rows
            .first()
            .is_some_and(|row| *row as usize >= population.0)
        || rows.last().is_some_and(|row| *row as usize >= population.0)
    {
        return Err(OperatingError::ShapeMismatch);
    }
    let rows = acquired
        .storage()
        .payload()
        .share_positions()
        .ok_or(OperatingError::WrongOperation)?;
    Ok(SelectedPositions(Arc::clone(rows.backing())))
}
