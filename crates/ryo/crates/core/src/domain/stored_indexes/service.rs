//! Budgeted acquisition of complete typed indexes, with invocation-local fallback.
use super::types::{Acquired, Entry, FileIndexes};
use crate::domain::coverage::types::Partial;
use crate::domain::table_cells::types::RowIndex;
use crate::domain::table_model::{
    errors::TableModelError,
    service::{check_columns, index_stored},
    types::{ChainLink, HashedRow, KeyCellsHash, KeyIndex},
};
use ryo_language::domain::relations::types::{ColumnsView, Relation};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use std::sync::Arc;

/// Per-file retained index budget; invocation-local fallback retains its existing policy.
pub(crate) const LIMIT: usize = 16 * 1024 * 1024;

/// Charge the Arc payload/header and conservatively allow control words and alignment padding.
const ARC_BYTES: usize =
    size_of::<KeyIndex>() + 2 * size_of::<usize>() + align_of::<KeyIndex>() - 1;

/// Start one generation from the actual physical read state, not execution-mode coverage.
pub(crate) fn new(wanted: &[Relation], partials: &[Partial], limit: usize) -> Option<FileIndexes> {
    let requested = wanted
        .len()
        .checked_mul(size_of::<Relation>())?
        .checked_add(size_of::<FileIndexes>())?;
    if requested > limit {
        return None;
    }
    let mut eligible = Vec::new();
    eligible.try_reserve_exact(wanted.len()).ok()?;
    for relation in wanted {
        if !partials.iter().any(|partial| partial.relation == *relation)
            && !eligible.contains(relation)
        {
            eligible.push(*relation);
        }
    }
    let bytes = size_of::<FileIndexes>()
        .checked_add(eligible.capacity().checked_mul(size_of::<Relation>())?)?;
    (bytes <= limit).then_some(FileIndexes {
        graph_enabled: false,
        graph_bytes: 0,
        memo_bytes: 0,
        graphs: Vec::new(),
        graph_domains: Vec::new(),
        nearest: Vec::new(),
        graph_counts: crate::domain::stored_graph::types::GraphCounts {
            peak_bytes: bytes,
            ..Default::default()
        },
        selectors: Vec::new(),
        selections: Vec::new(),
        discriminator_columns: Vec::new(),
        selection_counts: Default::default(),
        selection_histogram:
            tracing::enabled!(target: "ryo::diagnostic::selection_detail", tracing::Level::TRACE)
                .then(Vec::new),
        eligible,
        entries: Vec::new(),
        bytes,
        limit,
        builds: 0,
        reuses: 0,
        refusals: 0,
    })
}

/// Every admitted payload competes under the same actual-use per-file cap; no allowance is stranded.
pub(crate) fn non_graph_limit(cache: &FileIndexes) -> usize {
    cache.limit
}

/// Prepare bounded equality demand; duplicate projections alone cannot justify a new index.
pub(crate) fn candidate_columns(cache: &mut FileIndexes) {
    use crate::domain::stored_selection::types::CandidateColumn;
    use ryo_values::domain::plan_parts::types::{Operand, Test};
    let mut columns = std::collections::BTreeMap::new();
    for selector in &cache.selectors {
        for comparison in &selector.identity.comparisons {
            let ((Test::Equal, Operand::Column(column), _)
            | (Test::Equal, _, Operand::Column(column))) =
                (&comparison.test, &comparison.left, &comparison.right)
            else {
                continue;
            };
            let entry = columns
                .entry((selector.relation, *column))
                .or_insert_with(|| CandidateColumn {
                    relation: selector.relation,
                    column: *column,
                    first: Arc::clone(selector),
                    repeated: false,
                });
            entry.repeated |= entry.first.identity.comparisons != selector.identity.comparisons
                || entry.first.identity.bindings != selector.identity.bindings;
        }
    }
    let columns: Vec<_> = columns.into_values().collect();
    if let Some(bytes) = columns
        .capacity()
        .checked_mul(size_of::<CandidateColumn>())
        .and_then(|bytes| cache.bytes.checked_add(bytes))
        .filter(|bytes| *bytes <= non_graph_limit(cache))
    {
        cache.bytes = bytes;
        cache.graph_counts.peak_bytes = cache.graph_counts.peak_bytes.max(bytes);
        cache.discriminator_columns = columns;
    }
}

/// Sum actual capacities of every index vector and the Arc allocation metadata.
/// Stored indexes never allocate the grouping-only hash set; refuse rather than undercharge it.
pub(crate) fn index_bytes(index: &KeyIndex) -> Option<usize> {
    if index.runs.capacity() != 0 {
        return None;
    }
    let vectors = [
        (index.rows.capacity(), size_of::<RowIndex>()),
        (index.hashes.capacity(), size_of::<KeyCellsHash>()),
        (index.heads.capacity(), size_of::<ChainLink>()),
        (index.next.capacity(), size_of::<ChainLink>()),
        (index.sorted.capacity(), size_of::<RowIndex>()),
        (index.pairs.capacity(), size_of::<HashedRow>()),
    ];
    vectors
        .into_iter()
        .try_fold(ARC_BYTES, |bytes, (capacity, width)| {
            bytes.checked_add(capacity.checked_mul(width)?)
        })
}

/// A lower bound avoids building a second, unretainable index when the budget clearly cannot fit it.
fn minimum_bytes(rows: usize, columns: usize) -> Option<usize> {
    let buckets = rows.checked_next_power_of_two()?.max(1);
    ARC_BYTES
        .checked_add(rows.checked_mul(
            size_of::<RowIndex>() + size_of::<KeyCellsHash>() + size_of::<ChainLink>(),
        )?)?
        .checked_add(buckets.checked_mul(size_of::<ChainLink>())?)?
        .checked_add(columns.checked_mul(size_of::<ColumnIndex>())?)?
        .checked_add(size_of::<Entry>())
}

/// A selector should scan instead of building an index that cannot possibly be shared.
pub(crate) fn can_retain(
    cache: &FileIndexes,
    relation: Relation,
    view: ColumnsView<'_>,
    columns: &[ColumnIndex],
) -> bool {
    let rows = view.columns.first().map_or(0, |column| column.0.len());
    cache.entries.iter().any(|entry| {
        entry.relation == relation
            && entry.columns == columns
            && entry.kinds == view.kinds
            && entry.rows == rows
    }) || minimum_bytes(rows, columns.len())
        .and_then(|bytes| cache.bytes.checked_add(bytes))
        .is_some_and(|bytes| bytes <= non_graph_limit(cache))
}

/// Build the existing invocation-local path when sharing is unavailable or refused.
fn local(
    cache: &mut FileIndexes,
    view: ColumnsView<'_>,
    columns: &[ColumnIndex],
    into: &mut KeyIndex,
    eligible: bool,
) -> Result<Acquired, TableModelError> {
    index_stored(view, columns, into)?;
    cache.builds = cache.builds.saturating_add(1);
    if eligible {
        cache.refusals = cache.refusals.saturating_add(1);
    }
    Ok(Acquired {
        shared: None,
        built: true,
        eligible,
    })
}

/// Prepare an index after the provider read and first left row have already succeeded.
/// Fully built indexes alone are admitted; returned Arc handles must remain within the file read.
///
/// # Errors
///
/// The existing typed index's column validation error; a hit never bypasses validation.
pub(crate) fn acquire(
    cache: &mut FileIndexes,
    relation: Relation,
    view: ColumnsView<'_>,
    columns: &[ColumnIndex],
    into: &mut KeyIndex,
) -> Result<Acquired, TableModelError> {
    check_columns(view.kinds, columns)?;
    if !cache.eligible.contains(&relation) {
        return local(cache, view, columns, into, false);
    }
    let rows = view.columns.first().map_or(0, |column| column.0.len());
    if let Some(entry) = cache.entries.iter().find(|entry| {
        entry.relation == relation
            && entry.columns.as_slice() == columns
            && entry.kinds == view.kinds
            && entry.rows == rows
    }) {
        let index = Arc::clone(&entry.index);
        cache.reuses = cache.reuses.saturating_add(1);
        return Ok(Acquired {
            shared: Some(index),
            built: false,
            eligible: true,
        });
    }
    if minimum_bytes(rows, columns.len())
        .and_then(|more| cache.bytes.checked_add(more))
        .is_none_or(|bytes| bytes > non_graph_limit(cache))
    {
        return local(cache, view, columns, into, true);
    }
    build(cache, relation, view, columns, into, rows)
}

/// Construct complete data and admit only its accounted allocations; refuse without rebuilding.
fn build(
    cache: &mut FileIndexes,
    relation: Relation,
    view: ColumnsView<'_>,
    columns: &[ColumnIndex],
    into: &mut KeyIndex,
    rows: usize,
) -> Result<Acquired, TableModelError> {
    let mut owned_columns = Vec::new();
    if owned_columns.try_reserve_exact(columns.len()).is_err() {
        return local(cache, view, columns, into, true);
    }
    owned_columns.extend_from_slice(columns);
    // Allocate replacement metadata separately, so a refused capacity increase is never retained.
    let mut replacement = None;
    if cache.entries.len() == cache.entries.capacity() {
        let mut entries = Vec::new();
        if entries
            .try_reserve_exact(cache.entries.len().saturating_add(1))
            .is_err()
        {
            return local(cache, view, columns, into, true);
        }
        replacement = Some(entries);
    }
    let mut index = KeyIndex::default();
    index_stored(view, columns, &mut index)?;
    cache.builds = cache.builds.saturating_add(1);
    let old_metadata = cache.entries.capacity().checked_mul(size_of::<Entry>());
    let new_capacity = replacement
        .as_ref()
        .map_or(cache.entries.capacity(), Vec::capacity);
    let retained = old_metadata
        .and_then(|old| cache.bytes.checked_sub(old))
        .and_then(|bytes| bytes.checked_add(new_capacity.checked_mul(size_of::<Entry>())?))
        .and_then(|bytes| {
            bytes.checked_add(
                owned_columns
                    .capacity()
                    .checked_mul(size_of::<ColumnIndex>())?,
            )
        })
        .and_then(|bytes| bytes.checked_add(index_bytes(&index)?));
    let Some(bytes) = retained.filter(|bytes| *bytes <= non_graph_limit(cache)) else {
        // The completed fresh index is still usable locally; never build it twice on refusal.
        *into = index;
        cache.refusals = cache.refusals.saturating_add(1);
        return Ok(Acquired {
            shared: None,
            built: true,
            eligible: true,
        });
    };
    if let Some(mut entries) = replacement {
        entries.append(&mut cache.entries);
        cache.entries = entries;
    }
    let index = Arc::new(index);
    cache.entries.push(Entry {
        relation,
        columns: owned_columns,
        kinds: view.kinds,
        rows,
        index: Arc::clone(&index),
    });
    cache.bytes = bytes;
    cache.graph_counts.peak_bytes = cache.graph_counts.peak_bytes.max(bytes);
    Ok(Acquired {
        shared: Some(index),
        built: true,
        eligible: true,
    })
}
