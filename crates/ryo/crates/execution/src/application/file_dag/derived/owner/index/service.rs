//! Budgeted acquisition of complete typed indexes, with invocation-local fallback.
use super::types::{ByteLimit, IndexBytes, IndexRowCount, KeyColumns};
use crate::application::file_dag::derived::constants as activation;
use crate::application::file_dag::derived::ledger::types::ByteCount;
use crate::application::file_dag::derived::owner::cache::types::CacheBackend;
use crate::application::file_dag::derived::owner::selection::types::Retainability;
use crate::domain::coverage::types::Partial;

use ryo_derived::domain::contract::types::{ChainLink, KeyCellsHash, KeyIndex};
use ryo_relations::domain::relations::types::{ColumnsView, Relation};
use ryo_values::domain::plan_parts::types::ColumnIndex;

/// Per-file retained index budget; invocation-local fallback retains its existing policy.
pub(crate) const LIMIT: usize = 16 * 1024 * 1024;

/// Charge the Arc payload/header and conservatively allow control words and alignment padding.
const ARC_BYTES: usize =
    size_of::<KeyIndex>() + 2 * size_of::<usize>() + align_of::<KeyIndex>() - 1;

/// Start one generation from the actual physical read state, not execution-mode coverage.
pub(in crate::application::file_dag::derived::owner) fn new(
    wanted: &[Relation],
    partials: &[Partial],
    limit: ByteLimit,
) -> Option<CacheBackend> {
    let requested = wanted
        .len()
        .checked_mul(size_of::<Relation>())?
        .checked_add(size_of::<CacheBackend>())?;
    if requested > limit.0 {
        return None;
    }
    let mut eligible = Vec::new();
    if eligible.try_reserve_exact(wanted.len()).is_err() {
        return None;
    }
    for relation in wanted {
        if !partials.iter().any(|partial| partial.relation == *relation)
            && !eligible.contains(relation)
        {
            eligible.push(*relation);
        }
    }
    let bytes =
        crate::application::file_dag::derived::ledger::capacity::headed::<CacheBackend, Relation>(
            crate::application::file_dag::derived::ledger::types::SlotCount(eligible.capacity()),
        )?
        .0;
    (bytes <= limit.0).then_some(CacheBackend {
        #[cfg(test)]
        selector_descriptor: None,
        selector_repetition: std::collections::HashMap::new(),
        selector_repetition_failed:
            crate::application::file_dag::derived::owner::cache::types::RepetitionTracking::Tracked,
        selection_views: None,
        graph_enabled:
            crate::application::file_dag::derived::owner::cache::types::GraphAdmission::Disabled,
        graph_bytes: ByteCount(0),
        memo_bytes: ByteCount(0),
        graphs: Vec::new(),
        graph_domains: Vec::new(),
        nearest: Vec::new(),
        graph_counts: crate::domain::stored_graph::types::GraphCounts {
            peak_bytes: bytes,
            ..Default::default()
        },
        selectors: Vec::new(),
        active_postings: Vec::new(),
        discriminator_columns: Vec::new(),
        selection_counts: Default::default(),
        selection_histogram:
            tracing::enabled!(target: activation::TARGET_SELECTION_DETAIL, tracing::Level::TRACE)
                .then(Vec::new),
        eligible,
        bytes: ByteCount(bytes),
        limit,
        builds: Default::default(),
        reuses: Default::default(),
        refusals: Default::default(),
    })
}

/// Every admitted payload competes under the same actual-use per-file cap; no allowance is stranded.
pub(in crate::application::file_dag::derived::owner) fn non_graph_limit(
    cache: &CacheBackend,
) -> ByteLimit {
    ByteLimit(cache.limit.0)
}

/// Prepare bounded equality demand; duplicate projections alone cannot justify a new index.
pub(in crate::application::file_dag::derived::owner) fn candidate_columns(
    cache: &mut CacheBackend,
) {
    use crate::domain::stored_selection::types::CandidateColumn;
    use ryo_values::domain::plan_parts::types::{Operand, Test};
    let mut columns = std::collections::BTreeMap::new();
    for selector in &cache.selectors {
        for comparison in &selector.identity.comparisons {
            tracing::trace!(target: activation::TARGET, event = activation::CANDIDATE);
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
                    first: (selector).clone(),
                    repeated: false,
                });
            entry.repeated |= entry.first.identity.comparisons != selector.identity.comparisons
                || entry.first.identity.bindings != selector.identity.bindings;
        }
    }
    let columns: Vec<_> = columns.into_values().collect();
    if let Some(bytes) =
        crate::application::file_dag::derived::ledger::capacity::checked::<CandidateColumn>(
            crate::application::file_dag::derived::ledger::types::SlotCount(columns.capacity()),
        )
        .and_then(|bytes| cache.bytes.0.checked_add(bytes.0))
        .filter(|bytes| *bytes <= non_graph_limit(cache).0)
    {
        cache.bytes.0 = bytes;
        cache.graph_counts.peak_bytes = cache.graph_counts.peak_bytes.max(bytes);
        cache.discriminator_columns = columns;
    }
}

/// A lower bound avoids building a second, unretainable index when the budget clearly cannot fit it.
pub(in crate::application::file_dag::derived::owner) fn minimum_bytes(
    rows: IndexRowCount,
    columns: KeyColumns,
) -> Option<IndexBytes> {
    let rows = rows.0;
    let buckets = rows.checked_next_power_of_two()?.max(1);
    ARC_BYTES
        .checked_add(rows.checked_mul(
            size_of::<RowIndex>() + size_of::<KeyCellsHash>() + size_of::<ChainLink>(),
        )?)?
        .checked_add(buckets.checked_mul(size_of::<ChainLink>())?)?
        .checked_add(columns.0.checked_mul(size_of::<ColumnIndex>())?)
        .map(IndexBytes)
}

/// A selector should scan instead of building an index that cannot possibly be shared.
pub(in crate::application::file_dag::derived::owner) fn can_retain(
    cache: &CacheBackend,
    relation: Relation,
    view: ColumnsView<'_>,
    columns: &[ColumnIndex],
) -> Retainability {
    let rows = view.columns.first().map_or(0, |column| column.0.len());
    let fits = minimum_bytes(IndexRowCount(rows), KeyColumns(columns.len()))
        .and_then(|bytes| cache.bytes.0.checked_add(bytes.0))
        .is_some_and(|bytes| bytes <= non_graph_limit(cache).0);
    if fits {
        Retainability::Retainable
    } else {
        Retainability::Unretainable
    }
}

use ryo_values::domain::rows::types::RowIndex;
