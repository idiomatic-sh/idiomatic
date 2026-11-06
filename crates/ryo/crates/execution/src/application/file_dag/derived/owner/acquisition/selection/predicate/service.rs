//! Owner-bound predicate construction over a chosen candidate set.
use super::super::super::super::*;
use super::super::{
    service::{SelectionSource, histogram, shared_positions},
    types::{PredicateDecision, RowBound, SelectorGroup, Visits},
};
use crate::application::file_dag::derived::owner::selection::types::{
    CandidateChoice, SelectionPath,
};
use crate::domain::{
    operating::types::{Memo, NameCount, OperatorContext},
    operator::contract::views::BorrowedRows,
    stored_selection::types::{PreparedSelectorViews, Selector},
    table_model::service::check_columns,
};
use crate::application::file_dag::derived::owner::acquisition::population::types::Construction;
use ryo_language::domain::symbols::service::name_count;
use ryo_relations::domain::relations::types::ColumnsView;
use ryo_values::domain::plan_parts::types::ColumnIndex;

type CandidateChooser = fn(
    &mut CacheBackend,
    SelectionSource<'_>,
    &Selector,
    &PreparedSelectorViews,
    ColumnsView<'_>,
    &OperatorContext<'_>,
) -> Result<CandidateChoice, OperatingError>;

pub(in crate::application::file_dag::derived::owner) fn matches(
    cache: &mut CacheBackend,
    resources: SelectionSource<'_>,
    program: &Arc<Selector>,
    views: &PreparedSelectorViews,
    source: ColumnsView<'_>,
    context: &mut OperatorContext<'_>,
    choose: CandidateChooser,
) -> Result<
    crate::application::file_dag::derived::owner::acquisition::admission::service::AcquiredView,
    OperatingError,
> {
    let acquired = resources.acquire_predicate(
        cache,
        &views.predicate,
        SelectorGroup(views.group),
        |cache| build_matches(cache, resources, program, views, source, context, choose),
    )?;
    let population = source.columns.first().map_or(0, |column| column.0.len());
    let rows = shared_positions(&acquired, RowBound(population), RowBound(population))?.0;
    if let Some(entry) = histogram(cache, program) {
        let built = acquired.construction() == Construction::Built;
        entry.builds += u64::from(built);
        entry.reuses += u64::from(!built);
        if built {
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
    choose: CandidateChooser,
) -> Result<DerivedStorage, OperatingError> {
    let needed = &views.predicate.needed;
    // Preserve the old first invalid predicate-column error before candidate acquisition.
    for &column in &needed.0 {
        check_columns(
            source.kinds,
            &[ColumnIndex(u32::try_from(column).unwrap_or(u32::MAX))],
        )?;
    }
    let CandidateChoice {
        rows: candidates,
        path,
    } = choose(cache, resources, program, views, source, context)?;
    let indexed = candidates.is_some();
    let population = source.columns.first().map_or(0, |column| column.0.len());
    let names = NameCount(name_count(context.symbols.as_ref()).0);
    context.memo.clear();
    context.memo.resize(
        program.identity.comparisons.len().saturating_mul(names.0),
        Memo::Unknown,
    );
    let mut decision = PredicateDecision {
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
        visits: Visits(0),
    };
    let outcome = crate::application::file_dag::route_counts::observe(
        crate::application::file_dag::route_counts::BuildRoute::SelectorPredicate.label(),
        crate::application::file_dag::route_counts::derived(views.predicate.kind),
        || {
            resources.build(
                &views.predicate,
                DerivedInputs::Predicate {
                    rows: BorrowedRows::Stored(source),
                    candidates: candidates.as_ref().map(|rows| rows.as_slice().0),
                    program: &mut decision,
                    symbols: context.symbols.as_ref(),
                },
                &mut DerivedScratch::default(),
            )
        },
    );
    let visited = decision.visits.0;
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
                entry.indexed_candidates += candidates
                    .as_ref()
                    .map_or(0, |rows| rows.as_slice().0.len())
                    as u64;
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
