//! Owner-bound selector acquisition; immutable views leave this scope, never the backend.
use super::super::super::*;
use crate::application::file_dag::derived::owner::selection::candidates::candidates;
use crate::application::file_dag::derived::owner::selection::{
    CandidateRows, SelectionPath, histogram, shared_positions,
};
use crate::application::file_dag::relations::{AdmittedInput, RelationOwner};
use crate::domain::{
    operating::types::NameCount, stored_selection::types::PreparedSelectorViews,
    table_model::service::check_columns, table_texts::types::Texts,
};
use crate::domain::{operating::types::OperatorContext, stored_selection::types::Selector};
use ryo_language::domain::symbols::service::name_count;
use ryo_relations::domain::relations::types::ColumnsView;
use ryo_relations::domain::relations::types::{CellKind, Value};
#[derive(Clone, Copy)]
pub(in crate::application::file_dag::derived::owner) struct SelectionSource<'a> {
    owner: &'a DerivedOwner,
    source: &'a RelationOwner,
    proof: &'a AdmittedInput<'a>,
    population: PopulationId,
}
impl SelectionSource<'_> {
    fn build(
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
                if groups >= crate::application::file_dag::derived::owner::selection::MAX_FANOUT_GROUPS
                    || literals > crate::application::file_dag::derived::owner::selection::MAX_FANOUT_LITERALS
                    || text_bytes > crate::application::file_dag::derived::owner::selection::MAX_FANOUT_TEXT_BYTES
                    || population
                        .checked_mul(2)
                        .and_then(|need| units.checked_add(need))
                        .is_none_or(|units| {
                            units > crate::application::file_dag::derived::owner::selection::MAX_RETAINED_POSITION_CAPACITY
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

    pub(in crate::application::file_dag::derived::owner) fn can_retain_equality(
        &self,
        cache: &CacheBackend,
        relation: ryo_relations::domain::relations::types::Relation,
        source: ColumnsView<'_>,
        columns: &[ryo_values::domain::plan_parts::types::ColumnIndex],
    ) -> bool {
        let live_hint = self.owner.has_live_equality(source, columns);
        let allowed = live_hint
            || crate::application::file_dag::derived::owner::index::service::can_retain(
                cache, relation, source, columns,
            );
        if !allowed {
            if tracing::enabled!(target: "ryo::diagnostic::selection_detail", tracing::Level::TRACE)
            {
                self.owner.report_selector_occupancy(self.population);
            }
            tracing::trace!(target: "ryo::diagnostic::selection_detail", event = "selector_equality_admission_refused",
                population = ?self.population, relation = ?relation, columns = ?columns,
                rows = source.columns.first().map_or(0, |column| column.0.len()),
                cache_bytes = cache.bytes, cache_limit = cache.limit,
                minimum_index_bytes = crate::application::file_dag::derived::owner::index::service::minimum_bytes(source.columns.first().map_or(0, |column| column.0.len()), columns.len()),
                live_hint, ledger = ?self.owner.ledger.borrow().snapshot());
        }
        allowed
    }

    fn acquire_predicate(
        &self,
        cache: &mut CacheBackend,
        plan: &Arc<DerivedContract>,
        group: usize,
        build: impl FnOnce(&mut CacheBackend) -> Result<DerivedStorage, OperatingError>,
    ) -> Result<
        crate::application::file_dag::derived::owner::acquisition::admission::service::AcquiredView,
        OperatingError,
    > {
        if cache
            .selection_views
            .as_ref()
            .is_none_or(|views| group >= views.groups)
        {
            return Err(OperatingError::NotRun);
        }
        let repeated = cache.selector_repetition.get(&group).copied().unwrap_or(0);
        let optional = !cache.selector_repetition_failed && repeated >= 2;
        let admitted = self.owner.admit_view(plan, self.source, &[self.proof])?;
        self.owner.acquire_with(
            plan,
            &admitted,
            ViewContext::default(),
            false,
            Some(cache),
            |cache| {
                let cache = cache.ok_or(OperatingError::NotRun)?;
                Ok((build(cache)?, true, false, optional))
            },
        )
    }
    pub(in crate::application::file_dag::derived::owner) fn acquire(
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
        relation_owner: &RelationOwner,
        publication: &crate::application::file_dag::relations::PublishedInput,
        proof: &AdmittedInput<'_>,
        context: &mut OperatorContext<'_>,
    ) -> Result<Option<crate::application::file_dag::relations::PublishedInput>, OperatingError>
    {
        proof.require_publication(relation_owner, publication.as_ref())?;
        if !std::ptr::eq(self, &relation_owner.derived)
            || !crate::application::file_dag::relations::same_admitted_rows(
                proof.rows(),
                crate::domain::operator::contract::views::BorrowedRows::Stored(source),
            )
        {
            return Err(OperatingError::NotRead);
        }
        let population = proof.identity(relation_owner)?.0;
        let produced = {
            let mut backend = self.backend.borrow_mut();
            let Some(backend) = backend.as_mut() else {
                return Ok(None);
            };
            crate::application::file_dag::derived::owner::selection::select(
                backend,
                SelectionSource {
                    owner: self,
                    source: relation_owner,
                    proof,
                    population,
                },
                program,
                source,
                context,
            )?
        };
        let Some(produced) = produced else {
            return Ok(None);
        };
        crate::application::file_dag::relations::selection::publish(
            relation_owner,
            publication,
            proof,
            produced.rows.selection,
            produced.resources,
        )
        .map(Some)
    }
}

// Borrow comparison state and count actual visits around the mandatory predicate item.

use ryo_language::domain::{files::types::FilePath, symbols::types::Symbols};
use ryo_values::domain::{plan_parts::types::Comparison, stored_facts::types::ArenaBytes};
use {
    crate::domain::comparing::service::holds_bound, crate::domain::operating::types::Memo,
    crate::domain::operator::contract::types::PredicateProgram,
    crate::domain::table_scratch::types::SelectScratch,
    ryo_values::domain::files::types::SourceView,
};
struct Decision<'a> {
    comparisons: &'a [Comparison],
    kinds: &'a [CellKind],
    names: NameCount,
    source: SourceView<'a>,
    paths: &'a [FilePath],
    arenas: &'a [ArenaBytes],
    bindings: &'a [ParameterBinding],
    prepared: Option<&'a crate::domain::matching::types::prepared::value::PreparedMatchers>,
    select: &'a mut SelectScratch,
    memo: &'a mut Vec<Memo>,
    visits: u64,
}
impl PredicateProgram for Decision<'_> {
    fn holds(&mut self, row: &[Value], symbols: &Symbols) -> Result<Verdict, OperatingError> {
        self.visits += 1;
        let texts = Texts {
            symbols,
            source: self.source,
            paths: self.paths,
            arenas: self.arenas,
        };
        holds_bound(
            self.comparisons,
            row,
            (self.kinds, self.names),
            &texts,
            self.bindings,
            self.prepared,
            (&mut *self.select, &mut *self.memo),
        )
    }
}
pub(in crate::application::file_dag::derived::owner) fn matches(
    cache: &mut CacheBackend,
    resources: SelectionSource<'_>,
    program: &Arc<Selector>,
    views: &PreparedSelectorViews,
    source: ColumnsView<'_>,
    context: &mut OperatorContext<'_>,
) -> Result<
    crate::application::file_dag::derived::owner::acquisition::admission::service::AcquiredView,
    OperatingError,
> {
    let acquired = resources.acquire_predicate(cache, &views.predicate, views.group, |cache| {
        build_matches(cache, resources, program, views, source, context)
    })?;
    let population = source.columns.first().map_or(0, |column| column.0.len());
    let rows = shared_positions(&acquired, population, population)?;
    if let Some(entry) = histogram(cache, program) {
        entry.builds += u64::from(acquired.was_built());
        entry.reuses += u64::from(!acquired.was_built());
        if acquired.was_built() {
            entry.matched_rows += rows.len() as u64;
        }
    }
    Ok(acquired)
}
fn build_matches(
    cache: &mut CacheBackend,
    resources: SelectionSource<'_>,
    program: &Arc<Selector>,
    views: &PreparedSelectorViews,
    source: ColumnsView<'_>,
    context: &mut OperatorContext<'_>,
) -> Result<DerivedStorage, OperatingError> {
    let needed = &views.predicate.needed;
    // Preserve the old first invalid predicate-column error before candidate acquisition.
    for &column in &needed.0 {
        check_columns(
            source.kinds,
            &[ColumnIndex(u32::try_from(column).unwrap_or(u32::MAX))],
        )?;
    }
    let (candidates, path) = candidates(cache, resources, program, views, source, context)?;
    let indexed = candidates.is_some();
    let population = source.columns.first().map_or(0, |column| column.0.len());
    let names = NameCount(name_count(context.symbols.as_ref()).0);
    context.memo.clear();
    context.memo.resize(
        program.identity.comparisons.len().saturating_mul(names.0),
        Memo::Unknown,
    );
    let mut decision = Decision {
        comparisons: &program.identity.comparisons,
        kinds: source.kinds,
        names,
        source: context.source,
        paths: context.paths,
        arenas: context.arenas,
        bindings: &program.identity.bindings,
        prepared: context.prepared_matchers,
        select: &mut *context.scratch.select_scratch,
        memo: &mut *context.memo,
        visits: 0,
    };
    let outcome = crate::application::file_dag::route_counts::observe(
        crate::application::file_dag::route_counts::BuildRoute::SelectorPredicate.label(),
        crate::application::file_dag::route_counts::derived(views.predicate.kind),
        || {
            resources.build(
                &views.predicate,
                DerivedInputs::Predicate {
                    rows: BorrowedRows::Stored(source),
                    candidates: candidates.as_ref().map(CandidateRows::as_slice),
                    program: &mut decision,
                    symbols: context.symbols.as_ref(),
                },
                &mut DerivedScratch::default(),
            )
        },
    );
    let visited = decision.visits;
    cache.selection_counts.predicate_rows = cache
        .selection_counts
        .predicate_rows
        .saturating_add(visited);
    if !indexed {
        cache.selection_counts.scanned = cache.selection_counts.scanned.saturating_add(visited);
    }
    cache.selection_counts.predicate_cells += visited * needed.0.len() as u64;
    if cache.selection_histogram.is_some() {
        if let Some(entry) = histogram(cache, program) {
            entry.paths[path as usize] += 1;
            if indexed {
                entry.indexed_candidates +=
                    candidates.as_ref().map_or(0, |rows| rows.as_slice().len()) as u64;
                if matches!(path, SelectionPath::IndexBuilt | SelectionPath::FanoutBuilt) {
                    entry.index_build_rows += population as u64;
                }
            } else {
                entry.full_scan_rows += visited;
            }
            entry.predicate_cells += visited * needed.0.len() as u64;
            entry.errors += u64::from(outcome.is_err());
        }
    }
    outcome
}

use crate::domain::{operator::contract::views::BorrowedRows, table_model::types::Verdict};
use ryo_values::domain::{plan_parts::types::ColumnIndex, query_bindings::types::ParameterBinding};
