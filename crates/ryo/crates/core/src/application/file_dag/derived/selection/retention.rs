//! Bounded admission into the existing once-charged match-position backend.
use super::*;
/// Retain only repeated results inside one fixed file-local selection allowance.
pub(super) fn admit(
    cache: &mut FileIndexes,
    program: &Arc<Selector>,
    views: &PreparedSelectorViews,
    source: ColumnsView<'_>,
    population: usize,
    rows: &Arc<Vec<u32>>,
) -> Result<Admission, OperatingError> {
    if cache.selector_repetition_failed {
        cache.selection_counts.refusals += 1;
        return Ok(Admission::AllocationRefused);
    }
    let repeated = *cache
        .selector_repetition
        .get(views.group)
        .ok_or(OperatingError::NotRun)?;
    if repeated < 2 {
        return Ok(Admission::NotRepeated);
    }
    let positions = cache.retained_positions.checked_add(rows.capacity());
    if cache.selections.len() >= MAX_RETAINED_SELECTIONS
        || positions.is_none_or(|total| total > MAX_RETAINED_POSITION_CAPACITY)
    {
        cache.selection_counts.refusals += 1;
        return Ok(Admission::BudgetRefused);
    }
    let first = cache.selections.is_empty() && cache.fanouts.is_empty();
    let reserved = first.then_some(RETAINED_SELECTION_RESERVATION).unwrap_or(0);
    let Some(bytes) = cache
        .bytes
        .checked_add(reserved)
        .filter(|bytes| *bytes <= super::super::indexes::non_graph_limit(cache))
    else {
        cache.selection_counts.refusals += 1;
        return Ok(Admission::BudgetRefused);
    };
    if cache.selections.try_reserve(1).is_err() {
        cache.selection_counts.refusals += 1;
        return Ok(Admission::AllocationRefused);
    }
    cache.selections.push(Matches {
        selector: Arc::clone(program),
        kinds: source.kinds,
        population,
        rows: Arc::clone(rows),
    });
    cache.retained_positions = positions.expect("checked retained position capacity");
    cache.bytes = bytes;
    cache.graph_counts.peak_bytes = cache.graph_counts.peak_bytes.max(bytes);
    Ok(Admission::Retained)
}
