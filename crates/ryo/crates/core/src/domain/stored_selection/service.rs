//! Demand-driven, generation-local compact selection and equality-candidate replay.

use super::types::program::value::SelectionProgram;
use super::types::{LiteralFanout, LiteralPostings, Matches, Selector, SelectorHistogram};
use crate::domain::comparing::service::holds_all;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::{NameCount, OperatorContext};
use crate::domain::operator::types::PreparedPool;
use crate::domain::stored_indexes::types::FileIndexes;
use crate::domain::table_cells::service::read_cells;
use crate::domain::table_cells::types::{CellsRead, ColumnPlaces, RowIndex};
use crate::domain::table_model::service::{
    check_columns, matching_stored, mix, rectangular, word_of,
};
use crate::domain::table_model::types::{
    KeyCellsHash, KeyIndex, ParameterBinding, SelectedRows, StoredSelection, Verdict,
};
use ryo_language::domain::relations::service::from_bits;
use ryo_language::domain::relations::types::{Bits, CellKind, ColumnsView, Flag, Integer, Value};
use ryo_language::domain::symbols::service::{find_hashed, hash_of, name_count, text_of};
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_language::domain::terms::service::symbols_with_text;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Operand, QueryLiteral, Test};
use ryo_values::domain::plans::types::Plan;
use std::sync::Arc;

// One file owns this cache and drops it after the file. Reserve a fixed, conservative
// slice of its existing limit for at most this many retained match positions and
// entries; no allocation-size estimate is maintained for individual entries.
pub(super) const RETAINED_SELECTION_RESERVATION: usize = 4 * 1024 * 1024;
const MAX_RETAINED_SELECTIONS: usize = 128;
pub(super) const MAX_RETAINED_POSITION_CAPACITY: usize = 512 * 1024;
// A single pass can route several authored literals without a whole-column key index.
// The position and metadata bounds fit inside the existing 4 MiB selection reservation.
const MAX_FANOUT_POPULATION: usize = 256 * 1024;
const MAX_FANOUT_LITERALS: usize = 128;
const MAX_FANOUT_GROUPS: usize = 4;
const MAX_FANOUT_TEXT_BYTES: usize = 64 * 1024;
const MIN_FANOUT_LITERALS: usize = 8;

/// Diagnostic reasons are fixed integers so normal selection never formats strings.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum SelectionPath {
    NoDiscriminator = 0,
    NotRepeated = 1,
    SmallPopulation = 2,
    DenseSample = 3,
    BudgetRefused = 4,
    IndexBuilt = 5,
    IndexReused = 6,
    FanoutBuilt = 7,
    FanoutReused = 8,
}

enum CandidateRows {
    Owned(Vec<u32>),
    Shared(Arc<Vec<u32>>),
}

impl CandidateRows {
    fn as_slice(&self) -> &[u32] {
        match self {
            Self::Owned(rows) => rows,
            Self::Shared(rows) => rows.as_ref().as_slice(),
        }
    }
}

/// An absent authored symbol spelling can rule out every row without building an index.
/// Other unsupported predicates still need ordinary row evaluation.
enum Discriminator {
    Unsupported,
    Impossible,
    Indexable(ColumnIndex, Value),
}

#[derive(Clone, Copy)]
pub(super) enum Admission {
    NotRepeated = 0,
    AllocationRefused = 1,
    BudgetRefused = 2,
    Retained = 3,
}

fn histogram<'a>(
    cache: &'a mut FileIndexes,
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

/// Prepare request-specific metadata once per worker, without evaluating any relation.
pub(crate) fn prepare(pool: &mut PreparedPool, plans: &[Plan], bindings: &[Vec<ParameterBinding>]) {
    if pool.selection_plans.len() == plans.len()
        && plans
            .iter()
            .zip(bindings)
            .zip(&pool.selection_plans)
            .all(|((plan, bound), prepared)| {
                prepared.identity.0 == std::ptr::from_ref(plan).addr() && prepared.bound == *bound
            })
    {
        return;
    }
    let started = tracing::enabled!(target: "ryo::diagnostic::execution", tracing::Level::TRACE)
        .then(std::time::Instant::now);
    // A new prepared version cannot reuse graph identities from an earlier request.
    pool.file_graph = None;
    pool.selection_plans = plans
        .iter()
        .zip(bindings)
        .map(|(plan, bound)| SelectionProgram::new(plan, bound).prepare())
        .collect();
    if let Some(started) = started {
        tracing::trace!(target: "ryo::diagnostic::execution", phase = "stored_program_setup", elapsed_ns = started.elapsed().as_nanos() as u64,
            plans = pool.selection_plans.len(), selectors = pool.selection_plans.iter().map(|plan| plan.steps.iter().flatten().count()).sum::<usize>());
    }
}

/// Install only active descriptors, bounded with all other generation-local retained cache data.
pub(crate) fn activate(
    cache: &mut FileIndexes,
    pool: &PreparedPool,
    selectable: &[ryo_language::domain::relations::types::Relation],
    active: impl Fn(usize) -> bool,
) {
    let mut selectors = Vec::new();
    for (rule, plan) in pool.selection_plans.iter().enumerate() {
        if !active(rule) {
            continue;
        }
        for selector in plan.steps.iter().flatten() {
            if !cache.eligible.contains(&selector.relation)
                || !selectable.contains(&selector.relation)
            {
                continue;
            }
            let required = (selectors.len() + 1).checked_mul(size_of::<Arc<Selector>>());
            if required
                .and_then(|bytes| cache.bytes.checked_add(bytes))
                .is_none_or(|bytes| {
                    bytes > crate::domain::stored_indexes::service::non_graph_limit(cache)
                })
            {
                break;
            }
            selectors.push(Arc::clone(selector));
        }
    }
    let bytes = selectors.capacity().checked_mul(size_of::<Arc<Selector>>());
    if let Some(bytes) = bytes
        .and_then(|bytes| cache.bytes.checked_add(bytes))
        .filter(|bytes| *bytes <= crate::domain::stored_indexes::service::non_graph_limit(cache))
    {
        cache.bytes = bytes;
        cache.graph_counts.peak_bytes = cache.graph_counts.peak_bytes.max(bytes);
        cache.selectors = selectors;
        crate::domain::stored_indexes::service::candidate_columns(cache);
    }
}

/// Exact ordered predicates and referenced binding values; origins/projections are not result identity.
fn same(left: &Selector, right: &Selector) -> bool {
    left.relation == right.relation
        && left.identity.comparisons == right.identity.comparisons
        && left.identity.bindings == right.identity.bindings
}

/// Produce a compact handle only after the provider has been read successfully by the caller.
pub(crate) fn select<'a>(
    program: &Arc<Selector>,
    source: ColumnsView<'a>,
    context: &mut OperatorContext<'_>,
) -> Result<Option<SelectedRows<'a>>, OperatingError> {
    if !rectangular(source)
        || source
            .columns
            .first()
            .is_some_and(|column| column.0.len() > u32::MAX as usize)
    {
        return Ok(None);
    }
    let Some(cache) = context.stored_indexes.take() else {
        return Ok(None);
    };
    let result = if cache.eligible.contains(&program.relation) {
        make(cache, program, source, context)
    } else {
        Ok(None)
    };
    context.stored_indexes = Some(cache);
    result
}

/// Validate projected columns before cache acquisition and leave matching ownership independent of projection.
fn make<'a>(
    cache: &mut FileIndexes,
    program: &Arc<Selector>,
    source: ColumnsView<'a>,
    context: &mut OperatorContext<'_>,
) -> Result<Option<SelectedRows<'a>>, OperatingError> {
    check_columns(source.kinds, &program.columns)?;
    if let Some(entry) = histogram(cache, program) {
        entry.calls += 1;
    }
    let population = source.columns.first().map_or(0, |column| column.0.len());
    let rows = if let Some(found) = cache.selections.iter().find(|entry| {
        same(&entry.selector, program)
            && entry.kinds == source.kinds
            && entry.population == population
    }) {
        let rows = Arc::clone(&found.rows);
        cache.selection_counts.reuses += 1;
        if let Some(entry) = histogram(cache, program) {
            entry.reuses += 1;
        }
        rows
    } else {
        let rows = Arc::new(matches(cache, program, source, context)?);
        cache.selection_counts.builds += 1;
        let admission = admit(cache, program, source, population, &rows);
        if let Some(entry) = histogram(cache, program) {
            entry.builds += 1;
            entry.matched_rows += rows.len() as u64;
            entry.admission[admission as usize] += 1;
        }
        rows
    };
    // The shared result above always contains predicate matches, never distinct representatives.
    // Each terminal projection owns its representative IDs independently of other output maps.
    let rows = if program.distinct {
        let Some(rows) = representatives(source, &program.columns, &rows, projection_hash) else {
            return Ok(None);
        };
        Arc::new(rows)
    } else {
        rows
    };
    let kinds = program
        .columns
        .iter()
        .map(|column| source.kinds[column.0 as usize])
        .collect();
    Ok(Some(SelectedRows {
        source,
        selection: Arc::new(StoredSelection {
            relation: program.relation,
            rows,
            columns: program.columns.clone(),
            kinds,
            identity: Arc::clone(&program.identity),
        }),
    }))
}

/// Decode one projected cell without treating missing storage as a value.
fn projected_cell(source: ColumnsView<'_>, row: u32, column: ColumnIndex) -> Option<Value> {
    let place = usize::try_from(column.0).ok()?;
    Some(from_bits(
        *source.kinds.get(place)?,
        Bits(*source.columns.get(place)?.0.get(row as usize)?),
    ))
}

/// Hash only projected typed values, preserving repeated columns and their order.
fn projection_hash(source: ColumnsView<'_>, columns: &[ColumnIndex], row: u32) -> Option<u64> {
    columns
        .iter()
        .try_fold(KeyCellsHash(0), |hash, column| {
            Some(mix(hash, word_of(&projected_cell(source, row, *column)?)))
        })
        .map(|hash| hash.0)
}

/// Keep the first physical representative of each tuple; the equality index dies with this call.
/// Cached hashes allow growth without decoding previously admitted rows again.
pub(super) fn representatives(
    source: ColumnsView<'_>,
    columns: &[ColumnIndex],
    rows: &[u32],
    hash_of: impl Fn(ColumnsView<'_>, &[ColumnIndex], u32) -> Option<u64>,
) -> Option<Vec<u32>> {
    if !rectangular(source) || check_columns(source.kinds, columns).is_err() {
        return None;
    }
    let mut seen = hashbrown::HashTable::<(u32, u64)>::new();
    let mut kept = Vec::new();
    for &row in rows {
        if source.columns.first()?.0.get(row as usize).is_none() {
            return None;
        }
        let hash = hash_of(source, columns, row)?;
        if seen
            .find(hash, |(previous, _)| {
                columns.iter().all(|column| {
                    projected_cell(source, *previous, *column)
                        .zip(projected_cell(source, row, *column))
                        .is_some_and(|(left, right)| left == right)
                })
            })
            .is_some()
        {
            continue;
        }
        let _ = seen.insert_unique(hash, (row, hash), |(_, hash)| *hash);
        kept.push(row);
    }
    Some(kept)
}

/// Retain only repeated results inside one fixed file-local selection allowance.
pub(super) fn admit(
    cache: &mut FileIndexes,
    program: &Arc<Selector>,
    source: ColumnsView<'_>,
    population: usize,
    rows: &Arc<Vec<u32>>,
) -> Admission {
    if cache
        .selectors
        .iter()
        .filter(|candidate| same(candidate, program))
        .take(2)
        .count()
        < 2
    {
        return Admission::NotRepeated;
    }
    let fanout_positions = cache.fanouts.iter().try_fold(0usize, |total, fanout| {
        fanout.literals.iter().try_fold(total, |total, posting| {
            total.checked_add(posting.rows.capacity())
        })
    });
    let positions = fanout_positions.and_then(|fanout| {
        cache
            .selections
            .iter()
            .try_fold(rows.capacity().checked_add(fanout)?, |total, entry| {
                total.checked_add(entry.rows.capacity())
            })
    });
    if cache.selections.len() >= MAX_RETAINED_SELECTIONS
        || positions.is_none_or(|total| total > MAX_RETAINED_POSITION_CAPACITY)
    {
        cache.selection_counts.refusals += 1;
        return Admission::BudgetRefused;
    }
    let first = cache.selections.is_empty() && cache.fanouts.is_empty();
    let reserved = first.then_some(RETAINED_SELECTION_RESERVATION).unwrap_or(0);
    let Some(bytes) = cache
        .bytes
        .checked_add(reserved)
        .filter(|bytes| *bytes <= crate::domain::stored_indexes::service::non_graph_limit(cache))
    else {
        cache.selection_counts.refusals += 1;
        return Admission::BudgetRefused;
    };
    if cache.selections.try_reserve(1).is_err() {
        cache.selection_counts.refusals += 1;
        return Admission::AllocationRefused;
    }
    cache.selections.push(Matches {
        selector: Arc::clone(program),
        kinds: source.kinds,
        population,
        rows: Arc::clone(rows),
    });
    cache.bytes = bytes;
    cache.graph_counts.peak_bytes = cache.graph_counts.peak_bytes.max(bytes);
    Admission::Retained
}

/// Decode only predicate cells; candidate indexes preserve original source order and exact typed equality.
fn matches(
    cache: &mut FileIndexes,
    program: &Arc<Selector>,
    source: ColumnsView<'_>,
    context: &mut OperatorContext<'_>,
) -> Result<Vec<u32>, OperatingError> {
    let mut needed = ColumnPlaces::default();
    crate::domain::text_gate::service::needed_of(&program.identity.comparisons, &mut needed);
    let columns: Vec<_> = needed
        .0
        .iter()
        .map(|index| ColumnIndex(u32::try_from(*index).unwrap_or(u32::MAX)))
        .collect();
    check_columns(source.kinds, &columns)?;
    let (candidates, path) = candidates(cache, program, source, context)?;
    let indexed = candidates.is_some();
    let mut buffer = vec![Value::Flag(Flag::Lowered); source.kinds.len()];
    let mut selected = Vec::new();
    let names = NameCount(name_count(context.symbols.as_ref()).0);
    let mut own = OperatorContext {
        prepared_kinds: context.prepared_kinds,
        prepared_matchers: context.prepared_matchers,
        stored_indexes: None,
        bindings: &program.identity.bindings,
        symbols: ryo_language::domain::symbols::service::query_reborrow(&mut context.symbols),
        source: context.source,
        paths: context.paths,
        arenas: context.arenas,
        memo: context.memo,
        scratch: context.scratch,
    };
    own.memo.clear();
    own.memo.resize(
        program.identity.comparisons.len().saturating_mul(names.0),
        crate::domain::operating::types::Memo::Unknown,
    );
    let population = source.columns.first().map_or(0, |column| column.0.len());
    let mut at = 0;
    let positions = std::iter::from_fn(|| {
        let position = match &candidates {
            Some(rows) => rows.as_slice().get(at).copied(),
            None => (at < population).then(|| u32::try_from(at).unwrap_or(u32::MAX)),
        };
        at += 1;
        position
    });
    let scanned_before = cache.selection_counts.scanned;
    let cells_before = cache.selection_counts.predicate_cells;
    let outcome: Result<(), OperatingError> = (|| {
        for position in positions {
            if !indexed {
                cache.selection_counts.scanned = cache.selection_counts.scanned.saturating_add(1);
            }
            cache.selection_counts.predicate_cells += needed.0.len() as u64;
            if read_cells(&source, RowIndex(position as usize), &mut buffer, &needed)
                == CellsRead::Read
                && holds_all(
                    &program.identity.comparisons,
                    &buffer,
                    source.kinds,
                    names,
                    &mut own,
                )? == Verdict::Holds
            {
                selected.push(position);
            }
        }
        Ok(())
    })();
    if cache.selection_histogram.is_some() {
        let visited = if indexed {
            (cache.selection_counts.predicate_cells - cells_before)
                .checked_div(needed.0.len() as u64)
                .unwrap_or(0)
        } else {
            cache.selection_counts.scanned - scanned_before
        };
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
    outcome.map(|()| selected)
}

/// Use a column index only when multiple active descriptors can amortize its build.
fn candidates(
    cache: &mut FileIndexes,
    program: &Selector,
    source: ColumnsView<'_>,
    context: &OperatorContext<'_>,
) -> Result<(Option<CandidateRows>, SelectionPath), OperatingError> {
    let population = source.columns.first().map_or(0, |column| column.0.len());
    if let Some((rows, path)) = fanout_candidates(cache, program, source, context) {
        cache.selection_counts.candidates += rows.len() as u64;
        return Ok((Some(CandidateRows::Shared(rows)), path));
    }
    let mut path = SelectionPath::NoDiscriminator;
    for (number, comparison) in program.identity.comparisons.iter().enumerate() {
        let (column, value) = match discriminator(comparison, program, source, context) {
            // holds_all evaluates in order. Only the first impossible comparison
            // may bypass rows without hiding an earlier comparison's error.
            Discriminator::Impossible if number == 0 => {
                return Ok((
                    Some(CandidateRows::Owned(Vec::new())),
                    SelectionPath::NoDiscriminator,
                ));
            }
            Discriminator::Indexable(column, value) => (column, value),
            Discriminator::Impossible | Discriminator::Unsupported => continue,
        };
        path = path.max(SelectionPath::NotRepeated);
        let reused = cache.discriminator_columns.iter().any(|entry| {
            entry.relation == program.relation && entry.column == column && entry.repeated
        });
        if !reused {
            continue;
        }
        if population < 32 {
            path = path.max(SelectionPath::SmallPopulation);
            continue;
        }
        if !selective(source, column, value, population) {
            path = path.max(SelectionPath::DenseSample);
            continue;
        }
        if !crate::domain::stored_indexes::service::can_retain(
            cache,
            program.relation,
            source,
            &[column],
        ) {
            path = path.max(SelectionPath::BudgetRefused);
            continue;
        }
        let mut local = KeyIndex::default();
        let acquired = crate::domain::stored_indexes::service::acquire(
            cache,
            program.relation,
            source,
            &[column],
            &mut local,
        )?;
        if acquired.built {
            cache.selection_counts.scanned += population as u64;
        }
        let order = acquired.shared.as_deref().unwrap_or(&local);
        let result: Vec<_> = matching_stored(source, &[column], order, &[value])
            .filter_map(|index| u32::try_from(index.0).ok())
            .collect();
        cache.selection_counts.candidates += result.len() as u64;
        return Ok((
            Some(CandidateRows::Owned(result)),
            if acquired.built {
                SelectionPath::IndexBuilt
            } else {
                SelectionPath::IndexReused
            },
        ));
    }
    Ok((None, path))
}

/// Only a total, authored, nonempty text equality in the first predicate can
/// reject rows before any caller-owned comparison or error origin is visited.
fn first_literal<'a>(selector: &'a Selector, kinds: &[CellKind]) -> Option<(ColumnIndex, &'a str)> {
    let comparison = selector.identity.comparisons.first()?;
    if comparison.test != Test::Equal {
        return None;
    }
    let (column, text) = match (&comparison.left, &comparison.right) {
        (Operand::Column(column), Operand::Literal(QueryLiteral::Text(text)))
        | (Operand::Literal(QueryLiteral::Text(text)), Operand::Column(column)) => {
            (column, &text.0)
        }
        _ => return None,
    };
    (kinds.get(column.0 as usize) == Some(&CellKind::Symbol) && !text.is_empty())
        .then_some((*column, text.as_ref()))
}

/// Route all active texts for one physical column in one pass. This is a
/// file-owned candidate index, never a cached predicate verdict: every owner
/// still evaluates its ordered comparisons and reports its own error origin.
fn fanout_candidates(
    cache: &mut FileIndexes,
    program: &Selector,
    source: ColumnsView<'_>,
    context: &OperatorContext<'_>,
) -> Option<(Arc<Vec<u32>>, SelectionPath)> {
    let (column, wanted) = first_literal(program, source.kinds)?;
    let place = column.0 as usize;
    let physical = source.columns.get(place)?.0.as_slice();
    let population = source.columns.first()?.0.len();
    if population < 32 || population > MAX_FANOUT_POPULATION {
        return None;
    }
    let source_bits = physical.as_ptr().addr();
    if let Some(entry) = cache.fanouts.iter().find(|entry| {
        entry.relation == program.relation
            && entry.column == column
            && entry.kinds == source.kinds
            && entry.population == population
            && entry.source_bits == source_bits
    }) {
        let at = entry
            .literals
            .binary_search_by(|item| item.text.as_str().cmp(wanted))
            .ok()?;
        return Some((
            Arc::clone(&entry.literals[at].rows),
            SelectionPath::FanoutReused,
        ));
    }
    if cache.fanouts.len() >= MAX_FANOUT_GROUPS {
        return None;
    }
    let prior_literals: usize = cache.fanouts.iter().map(|entry| entry.literals.len()).sum();
    let prior_text_bytes: usize = cache
        .fanouts
        .iter()
        .flat_map(|entry| &entry.literals)
        .map(|entry| entry.text.capacity())
        .sum();
    let mut texts = Vec::<String>::new();
    let mut text_bytes = prior_text_bytes;
    for selector in &cache.selectors {
        if selector.relation != program.relation {
            continue;
        }
        let Some((other_column, text)) = first_literal(selector, source.kinds) else {
            continue;
        };
        if other_column != column || texts.iter().any(|item| item.as_str() == text) {
            continue;
        }
        text_bytes = text_bytes.checked_add(text.len())?;
        if prior_literals + texts.len() >= MAX_FANOUT_LITERALS || text_bytes > MAX_FANOUT_TEXT_BYTES
        {
            return None;
        }
        texts.push(text.to_owned());
    }
    if texts.len() < MIN_FANOUT_LITERALS || !texts.iter().any(|text| text.as_str() == wanted) {
        return None;
    }
    // Vec growth can nearly double retained position capacity. Decline before
    // allocating if the worst case could exceed the existing selection slice.
    let prior_positions = cache
        .selections
        .iter()
        .map(|entry| entry.rows.capacity())
        .chain(
            cache
                .fanouts
                .iter()
                .flat_map(|entry| &entry.literals)
                .map(|entry| entry.rows.capacity()),
        )
        .try_fold(0usize, |sum, size| sum.checked_add(size))?;
    if population.checked_mul(2)? > MAX_RETAINED_POSITION_CAPACITY.checked_sub(prior_positions)? {
        return None;
    }
    let first = cache.selections.is_empty() && cache.fanouts.is_empty();
    let reserved = first.then_some(RETAINED_SELECTION_RESERVATION).unwrap_or(0);
    let bytes = cache.bytes.checked_add(reserved)?;
    if bytes > crate::domain::stored_indexes::service::non_graph_limit(cache) {
        return None;
    }
    texts.sort_unstable();
    let mut postings: Vec<_> = texts
        .into_iter()
        .map(|text| (text, Vec::<u32>::new()))
        .collect();
    for (row, bits) in physical.iter().enumerate() {
        let Value::Symbol(symbol) = from_bits(CellKind::Symbol, Bits(*bits)) else {
            return None;
        };
        // Malformed IDs have no nonempty text and cannot satisfy these total equalities.
        let Some(actual) = text_of(context.symbols.as_ref(), ByteSlice(context.source.0), symbol) else {
            continue;
        };
        if let Ok(at) = postings.binary_search_by(|entry| entry.0.as_str().cmp(actual.0)) {
            postings[at].1.push(u32::try_from(row).ok()?);
        }
    }
    let capacity = postings.iter().try_fold(prior_positions, |sum, entry| {
        sum.checked_add(entry.1.capacity())
    })?;
    if capacity > MAX_RETAINED_POSITION_CAPACITY || cache.fanouts.try_reserve(1).is_err() {
        return None;
    }
    let literals: Vec<_> = postings
        .into_iter()
        .map(|(text, rows)| LiteralPostings {
            text,
            rows: Arc::new(rows),
        })
        .collect();
    let at = literals
        .binary_search_by(|item| item.text.as_str().cmp(wanted))
        .ok()?;
    let result = Arc::clone(&literals[at].rows);
    cache.fanouts.push(LiteralFanout {
        relation: program.relation,
        column,
        kinds: source.kinds,
        population,
        source_bits,
        literals,
    });
    cache.bytes = bytes;
    cache.graph_counts.peak_bytes = cache.graph_counts.peak_bytes.max(bytes);
    cache.selection_counts.scanned += population as u64;
    Some((result, SelectionPath::FanoutBuilt))
}

/// A bounded typed sample rejects dense discriminators before allocating an index.
fn selective(
    source: ColumnsView<'_>,
    column: ColumnIndex,
    value: Value,
    population: usize,
) -> bool {
    let Some(kind) = source.kinds.get(column.0 as usize) else {
        return false;
    };
    let Some(values) = source.columns.get(column.0 as usize) else {
        return false;
    };
    let samples = population.min(64);
    let matches = (0..samples)
        .filter(|index| {
            values
                .0
                .get(index * population / samples)
                .is_some_and(|bits| from_bits(*kind, Bits(*bits)) == value)
        })
        .count();
    matches.saturating_mul(4) < samples
}

/// Equality lookup never interns query text and verifies symbol hashes by exact text.
fn discriminator(
    comparison: &ryo_values::domain::plan_parts::types::Comparison,
    program: &Selector,
    source: ColumnsView<'_>,
    context: &OperatorContext<'_>,
) -> Discriminator {
    if comparison.test != Test::Equal {
        return Discriminator::Unsupported;
    }
    let (column, operand) = match (&comparison.left, &comparison.right) {
        (Operand::Column(column), value) | (value, Operand::Column(column)) => (*column, value),
        _ => return Discriminator::Unsupported,
    };
    let literal = match operand {
        Operand::Literal(value) => value.clone(),
        Operand::Parameter(id) => {
            let Some(binding) = program.identity.bindings.get(id.0 as usize) else {
                return Discriminator::Unsupported;
            };
            match binding {
                ParameterBinding::Text(text) => QueryLiteral::Text(text.clone()),
                ParameterBinding::Number(number) => QueryLiteral::Number(Integer(number.0)),
                ParameterBinding::Flag(_)
                | ParameterBinding::Numbers(_)
                | ParameterBinding::Texts(_)
                | ParameterBinding::TextSets(_)
                | ParameterBinding::Tables(_) => return Discriminator::Unsupported,
            }
        }
        Operand::Column(_) | Operand::Function(_) => return Discriminator::Unsupported,
    };
    let value = match (source.kinds.get(column.0 as usize), literal) {
        (Some(CellKind::Number), QueryLiteral::Number(value)) => Value::Number(value),
        (Some(CellKind::Symbol), QueryLiteral::Text(text)) => {
            let hash = hash_of(ByteSlice(text.0.as_bytes()));
            // A file name and one or more closed-set terms can have the same text but
            // different Symbol IDs. The ordinary evaluator compares their text, while
            // matching_stored compares exact IDs. Index only an unambiguous spelling.
            let mut found_symbol = find_hashed(
                context.symbols.as_ref(),
                ByteSlice(context.source.0),
                ByteSlice(text.0.as_bytes()),
                hash,
            );
            let mut ambiguous = false;
            symbols_with_text(&text.0, |candidate| {
                if found_symbol.replace(candidate).is_some() {
                    ambiguous = true;
                }
            });
            if ambiguous {
                return Discriminator::Unsupported;
            }
            let Some(symbol) = found_symbol else {
                // Literal equality on Symbol rows compares text. Valid symbols
                // with this spelling would be in the dynamic or term lookup.
                // Keep empty text on the scan path: invalid symbols decode to
                // empty text in the ordinary evaluator. Bound parameters also
                // use the ordinary path, including their error behavior.
                return if matches!(operand, Operand::Literal(QueryLiteral::Text(_)))
                    && !text.0.is_empty()
                {
                    Discriminator::Impossible
                } else {
                    Discriminator::Unsupported
                };
            };
            Value::Symbol(symbol)
        }
        _ => return Discriminator::Unsupported,
    };
    Discriminator::Indexable(column, value)
}
