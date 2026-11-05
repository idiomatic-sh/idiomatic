//! Borrow comparison state and count actual visits around the mandatory predicate item.
use super::*;
use {crate::domain::comparing::service::holds_bound, crate::domain::operating::types::Memo, crate::domain::operator::contract::types::PredicateProgram, crate::domain::table_scratch::types::SelectScratch, ryo_values::domain::files::types::SourceView};
use ryo_language::domain::{files::types::FilePath, symbols::types::Symbols};
use ryo_values::domain::{plan_parts::types::Comparison, stored_facts::types::ArenaBytes};
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
pub(super) fn matches(
    cache: &mut CacheBackend,
    resources: super::super::owner::selection::SelectionSource<'_>,
    program: &Arc<Selector>,
    views: &PreparedSelectorViews,
    source: ColumnsView<'_>,
    context: &mut OperatorContext<'_>,
) -> Result<super::super::view::AcquiredView, OperatingError> {
    let acquired = resources.acquire_predicate(cache, &views.predicate, views.group, |cache| {
        build_matches(cache, resources, program, views, source, context)
    })?;
    let population = source.columns.first().map_or(0, |column| column.0.len());
    let rows = super::shared_positions(&acquired, population, population)?;
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
    resources: super::super::owner::selection::SelectionSource<'_>,
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
