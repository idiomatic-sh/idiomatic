//! Bounded admission into the existing once-charged match-position backend.
use super::*;
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
        .filter(|bytes| *bytes <= super::super::indexes::non_graph_limit(cache))
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
