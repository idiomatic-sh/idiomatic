//! Shared fanout policy declines are distinct from mandatory construction failures.
use super::*;
enum Choice {
    Reused(Arc<Vec<u32>>),
    Build {
        column: ColumnIndex,
        source_bits: usize,
        population: usize,
        prior_positions: usize,
        bytes: usize,
        texts: Vec<String>,
    },
}
fn choose(
    cache: &CacheBackend,
    program: &Selector,
    source: ColumnsView<'_>,
    context: &OperatorContext<'_>,
) -> Option<Choice> {
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
        return Some(Choice::Reused((entry.literals[at].rows).clone()));
    }
    if cache.fanouts.len() >= MAX_FANOUT_GROUPS {
        return None;
    }
    let prior_literals: usize = cache.fanouts.iter().map(|entry| entry.literals.len()).sum();
    let prior_text_bytes: usize = cache
        .fanouts
        .iter()
        .flat_map(|entry| entry.literals.iter())
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
    let prior_positions = cache.retained_positions;
    if population.checked_mul(2)? > MAX_RETAINED_POSITION_CAPACITY.checked_sub(prior_positions)? {
        return None;
    }
    let first = cache.selections.is_empty() && cache.fanouts.is_empty();
    let reserved = first.then_some(RETAINED_SELECTION_RESERVATION).unwrap_or(0);
    let bytes = cache.bytes.checked_add(reserved)?;
    if bytes > super::super::indexes::non_graph_limit(cache) {
        return None;
    }
    Some(Choice::Build {
        column,
        source_bits,
        population,
        prior_positions,
        bytes,
        texts,
    })
}
pub(super) fn fanout_candidates(
    cache: &mut CacheBackend,
    program: &Selector,
    plan: &DerivedContract,
    source: ColumnsView<'_>,
    context: &OperatorContext<'_>,
) -> Result<Option<(Arc<Vec<u32>>, SelectionPath)>, OperatingError> {
    let Some(choice) = choose(cache, program, source, context) else {
        return Ok(None);
    };
    let Choice::Build {
        column,
        source_bits,
        population,
        prior_positions,
        bytes,
        mut texts,
    } = choice
    else {
        let Choice::Reused(rows) = choice else {
            unreachable!()
        };
        return Ok(Some((rows, SelectionPath::FanoutReused)));
    };
    let (_, wanted) = first_literal(program, source.kinds).ok_or(OperatingError::WrongOperation)?;
    texts.sort_unstable();
    let active = texts
        .into_iter()
        .map(|text| LiteralText(text.into_boxed_str()))
        .collect::<Vec<_>>();
    let texts = Texts {
        symbols: context.symbols.as_ref(),
        source: context.source,
        paths: context.paths,
        arenas: context.arenas,
    };
    let mut storage = DerivedStorage::Unbuilt;
    super::super::owner::build_scoped(
        crate::application::file_dag::route_counts::BuildRoute::SelectorPostings,
        plan,
        DerivedInputs::Postings {
            rows: BorrowedRows::Stored(source),
            texts: &texts,
            active: &active,
        },
        &mut storage,
        &mut DerivedScratch::default(),
    )?;
    let crate::domain::derived_views::types::DerivedView::Postings(postings) =
        crate::domain::derived_views::collections::views::registration::registered(plan.kind)
            .view(plan, &storage)?
    else {
        return Err(OperatingError::WrongOperation);
    };
    let Some(capacity) = postings.iter().try_fold(prior_positions, |sum, entry| {
        sum.checked_add(entry.rows.capacity())
    }) else {
        return Ok(None);
    };
    if capacity > MAX_RETAINED_POSITION_CAPACITY || cache.fanouts.try_reserve(1).is_err() {
        return Ok(None);
    }
    let at = postings
        .binary_search_by(|item| item.text.as_str().cmp(wanted))
        .map_err(|_| OperatingError::WrongOperation)?;
    let result = (postings[at].rows).clone();
    let DerivedStorage::Postings(literals) = storage else {
        return Err(OperatingError::WrongOperation);
    };
    cache.fanouts.push(LiteralFanout {
        relation: program.relation,
        column,
        kinds: source.kinds,
        population,
        source_bits,
        literals,
    });
    cache.retained_positions = capacity;
    cache.bytes = bytes;
    cache.graph_counts.peak_bytes = cache.graph_counts.peak_bytes.max(bytes);
    cache.selection_counts.scanned += population as u64;
    Ok(Some((result, SelectionPath::FanoutBuilt)))
}
