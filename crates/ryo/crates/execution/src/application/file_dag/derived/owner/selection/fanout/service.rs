//! Active literal metadata is prepared once; only the common owner retains completed postings.
use super::super::*;
use crate::application::file_dag::derived::owner::cache::types::ActivePostings;
fn active_bytes(active: &Vec<ActivePostings>) -> Option<usize> {
    active
        .capacity()
        .checked_mul(size_of::<ActivePostings>())
        .and_then(|base| {
            active.iter().try_fold(base, |total, group| {
                let strings = group
                    .literals
                    .iter()
                    .try_fold(0_usize, |n, text| n.checked_add(text.0.len()))?;
                total
                    .checked_add(group.literals.len().checked_mul(size_of::<LiteralText>())?)?
                    .checked_add(2 * size_of::<usize>())?
                    .checked_add(strings)
            })
        })
}
pub(in crate::application::file_dag::derived::owner::selection) fn prepare_active(
    cache: &mut CacheBackend,
) {
    let previous = std::mem::take(&mut cache.active_postings);
    let Some(base) = active_bytes(&previous).and_then(|bytes| cache.bytes.checked_sub(bytes))
    else {
        return;
    };
    cache.bytes = base;
    drop(previous);
    let Some(views) = cache.selection_views.as_ref() else {
        return;
    };
    let mut groups = std::collections::BTreeMap::<(_, _), Vec<LiteralText>>::new();
    for selector in &cache.selectors {
        let Some(prepared) = views.by_selector.get(&(Arc::as_ptr(selector) as usize)) else {
            continue;
        };
        let Some(kinds) = prepared.postings.schemas.first() else {
            continue;
        };
        let Some((column, text)) = first_literal(selector, kinds) else {
            continue;
        };
        groups
            .entry((selector.relation, column))
            .or_default()
            .push(LiteralText(text.into()));
    }
    let mut active = Vec::new();
    for ((relation, column), mut literals) in groups {
        literals.sort_unstable();
        literals.dedup();
        active.push(ActivePostings {
            relation,
            column,
            literals: literals.into(),
        });
    }
    let bytes = active_bytes(&active);
    if let Some(bytes) = bytes
        .and_then(|bytes| cache.bytes.checked_add(bytes))
        .filter(|bytes| *bytes <= cache.limit)
    {
        cache.bytes = bytes;
        cache.graph_counts.peak_bytes = cache.graph_counts.peak_bytes.max(bytes);
        cache.active_postings = active;
    }
}
pub(in crate::application::file_dag::derived::owner::selection) fn fanout_candidates(
    cache: &mut CacheBackend,
    resources: crate::application::file_dag::derived::owner::acquisition::selection::service::SelectionSource<'_>,
    program: &Selector,
    plan: &Arc<DerivedContract>,
    source: ColumnsView<'_>,
    context: &OperatorContext<'_>,
) -> Result<
    Option<(
        Arc<Vec<u32>>,
        crate::application::file_dag::derived::owner::acquisition::admission::service::AcquiredView,
        SelectionPath,
    )>,
    OperatingError,
> {
    let Some((column, wanted)) = first_literal(program, source.kinds) else {
        return Ok(None);
    };
    let population = source.columns.first().map_or(0, |column| column.0.len());
    if !(32..=MAX_FANOUT_POPULATION).contains(&population) {
        return Ok(None);
    }
    let Some(active) = cache
        .active_postings
        .iter()
        .find(|entry| entry.relation == program.relation && entry.column == column)
        .map(|entry| Arc::clone(&entry.literals))
    else {
        return Ok(None);
    };
    if active.len() < MIN_FANOUT_LITERALS
        || active.len() > MAX_FANOUT_LITERALS
        || active
            .binary_search_by(|text| text.0.as_ref().cmp(wanted))
            .is_err()
    {
        return Ok(None);
    }
    let texts = Texts {
        symbols: context.symbols.as_ref(),
        source: context.source,
        paths: context.paths,
        arenas: context.arenas,
    };
    let Some(acquired) = resources.acquire_postings(cache, plan, source, &texts, active)? else {
        return Ok(None);
    };
    let rows = acquired
        .storage()
        .payload()
        .share_posting_positions(wanted)
        .ok_or(OperatingError::WrongOperation)?;
    if acquired.was_built() {
        cache.selection_counts.scanned += population as u64;
    }
    let path = if acquired.was_built() {
        SelectionPath::FanoutBuilt
    } else {
        SelectionPath::FanoutReused
    };
    Ok(Some((Arc::clone(rows.backing()), acquired, path)))
}
