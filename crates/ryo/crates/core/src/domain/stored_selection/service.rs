//! Demand-driven, generation-local compact selection and equality-candidate replay.

use super::types::program::value::SelectionProgram;
use super::types::{Matches, Selector};
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
use ryo_language::domain::symbols::service::{hash_in, hash_of, name_count, text_of};
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_language::domain::terms::types::Symbol;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Operand, QueryLiteral, Test};
use ryo_values::domain::plans::types::Plan;
use std::sync::Arc;

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
    let population = source.columns.first().map_or(0, |column| column.0.len());
    let rows = if let Some(found) = cache.selections.iter().find(|entry| {
        same(&entry.selector, program)
            && entry.kinds == source.kinds
            && entry.population == population
    }) {
        cache.selection_counts.reuses += 1;
        Arc::clone(&found.rows)
    } else {
        let rows = Arc::new(matches(cache, program, source, context)?);
        cache.selection_counts.builds += 1;
        admit(cache, program, source, population, &rows);
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

/// Retain only repeated results, charging vector capacities, Arc metadata and entry storage.
fn admit(
    cache: &mut FileIndexes,
    program: &Arc<Selector>,
    source: ColumnsView<'_>,
    population: usize,
    rows: &Arc<Vec<u32>>,
) {
    if cache
        .selectors
        .iter()
        .filter(|candidate| same(candidate, program))
        .take(2)
        .count()
        < 2
    {
        return;
    }
    let mut entries = Vec::new();
    if entries
        .try_reserve_exact(cache.selections.len() + 1)
        .is_err()
    {
        cache.selection_counts.refusals += 1;
        return;
    }
    let old = cache
        .selections
        .capacity()
        .saturating_mul(size_of::<Matches>());
    let retained = cache
        .bytes
        .checked_sub(old)
        .and_then(|bytes| bytes.checked_add(entries.capacity().checked_mul(size_of::<Matches>())?))
        .and_then(|bytes| bytes.checked_add(rows.capacity().checked_mul(size_of::<u32>())?))
        .and_then(|bytes| {
            bytes.checked_add(
                size_of::<Vec<u32>>() + 2 * size_of::<usize>() + align_of::<Vec<u32>>() - 1,
            )
        });
    let Some(bytes) = retained
        .filter(|bytes| *bytes <= crate::domain::stored_indexes::service::non_graph_limit(cache))
    else {
        cache.selection_counts.refusals += 1;
        return;
    };
    entries.append(&mut cache.selections);
    entries.push(Matches {
        selector: Arc::clone(program),
        kinds: source.kinds,
        population,
        rows: Arc::clone(rows),
    });
    cache.selections = entries;
    cache.bytes = bytes;
    cache.graph_counts.peak_bytes = cache.graph_counts.peak_bytes.max(bytes);
}

/// Decode only predicate cells; candidate indexes preserve original source order and exact typed equality.
fn matches(
    cache: &mut FileIndexes,
    program: &Selector,
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
    let candidates = candidates(cache, program, source, context)?;
    let indexed = candidates.is_some();
    let mut buffer = vec![Value::Flag(Flag::Lowered); source.kinds.len()];
    let mut selected = Vec::new();
    let names = NameCount(name_count(context.symbols).0);
    let mut own = OperatorContext {
        prepared_kinds: context.prepared_kinds,
        prepared_matchers: context.prepared_matchers,
        stored_indexes: None,
        bindings: &program.identity.bindings,
        symbols: context.symbols,
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
            Some(rows) => rows.get(at).copied(),
            None => (at < population).then(|| u32::try_from(at).unwrap_or(u32::MAX)),
        };
        at += 1;
        position
    });
    for position in positions {
        if !indexed {
            cache.selection_counts.scanned = cache.selection_counts.scanned.saturating_add(1);
        }
        cache.selection_counts.predicate_cells += needed.0.len() as u64;
        if read_cells(&source, RowIndex(position as usize), &mut buffer, &needed) == CellsRead::Read
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
    Ok(selected)
}

/// Use a column index only when multiple active descriptors can amortize its build.
fn candidates(
    cache: &mut FileIndexes,
    program: &Selector,
    source: ColumnsView<'_>,
    context: &OperatorContext<'_>,
) -> Result<Option<Vec<u32>>, OperatingError> {
    let population = source.columns.first().map_or(0, |column| column.0.len());
    for comparison in &program.identity.comparisons {
        let Some((column, value)) = discriminator(comparison, program, source, context) else {
            continue;
        };
        let reused = cache.discriminator_columns.iter().any(|entry| {
            entry.relation == program.relation && entry.column == column && entry.repeated
        });
        if !reused || population < 32 || !selective(source, column, value, population) {
            continue;
        }
        if !crate::domain::stored_indexes::service::can_retain(
            cache,
            program.relation,
            source,
            &[column],
        ) {
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
        return Ok(Some(result));
    }
    Ok(None)
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
                .is_some_and(|bits| {
                    ryo_language::domain::relations::service::from_bits(
                        *kind,
                        ryo_language::domain::relations::types::Bits(*bits),
                    ) == value
                })
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
) -> Option<(ColumnIndex, Value)> {
    if comparison.test != Test::Equal {
        return None;
    }
    let (column, operand) = match (&comparison.left, &comparison.right) {
        (Operand::Column(column), value) | (value, Operand::Column(column)) => (*column, value),
        _ => return None,
    };
    let literal = match operand {
        Operand::Literal(value) => value.clone(),
        Operand::Parameter(id) => match program.identity.bindings.get(id.0 as usize)? {
            ParameterBinding::Text(text) => QueryLiteral::Text(text.clone()),
            ParameterBinding::Number(number) => QueryLiteral::Number(Integer(number.0)),
            ParameterBinding::Flag(_)
            | ParameterBinding::Numbers(_)
            | ParameterBinding::Texts(_)
            | ParameterBinding::TextSets(_)
            | ParameterBinding::Tables(_) => return None,
        },
        Operand::Column(_) | Operand::Function(_) => return None,
    };
    let value = match (source.kinds.get(column.0 as usize)?, literal) {
        (CellKind::Number, QueryLiteral::Number(value)) => Value::Number(value),
        (CellKind::Symbol, QueryLiteral::Text(text)) => {
            let hash = hash_of(ByteSlice(text.0.as_bytes()));
            let symbol = (0..name_count(context.symbols).0).find_map(|index| {
                let symbol = Symbol(u32::try_from(index).ok()?);
                (hash_in(context.symbols, symbol) == Some(hash)
                    && text_of(context.symbols, ByteSlice(context.source.0), symbol)
                        .is_some_and(|found| found.0 == &*text.0))
                .then_some(symbol)
            })?;
            Value::Symbol(symbol)
        }
        _ => return None,
    };
    Some((column, value))
}
