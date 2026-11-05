//! Request preparation and bounded activation; never rebuild descriptors in a row path.
use super::*;
use crate::domain::operation_schema::types::InputShape;
use ryo_relations::domain::relations::types::Relation;
use std::collections::HashMap;

pub(crate) fn prepare_views_with_schemas<'s>(
    selectors: impl IntoIterator<Item = Arc<Selector>>,
    resolver: &dyn Fn(Relation) -> &'s [CellKind],
    semantic: Arc<dyn crate::outbound::derived::interface::DerivedPort>,
) -> Result<Arc<PreparedSelectionViews>, OperatingError> {
    let selectors = selectors.into_iter().collect::<Vec<_>>();
    let mut literals = HashMap::<(Relation, ColumnIndex), Vec<LiteralText>>::new();
    for selector in &selectors {
        if let Some((column, text)) = first_literal(selector, resolver(selector.relation)) {
            literals
                .entry((selector.relation, column))
                .or_default()
                .push(LiteralText(text.into()));
        }
    }
    for values in literals.values_mut() {
        values.sort_by(|left, right| left.0.cmp(&right.0));
        values.dedup_by(|left, right| left.0 == right.0);
    }
    let mut postings = HashMap::<(Relation, ColumnIndex), Arc<DerivedContract>>::new();
    let mut equalities = HashMap::<(Relation, ColumnIndex), Arc<DerivedContract>>::new();
    let mut prepared = PreparedSelectionViews {
        selectors,
        by_selector: HashMap::new(),
        groups: 0,
    };
    let mut groups: Vec<Arc<Selector>> = Vec::new();
    let mut predicates: Vec<Arc<DerivedContract>> = Vec::new();
    for selector in &prepared.selectors {
        let pointer = Arc::as_ptr(selector) as usize;
        if prepared.by_selector.contains_key(&pointer) {
            continue;
        }
        let group = groups
            .iter()
            .position(|previous| same(previous, selector))
            .unwrap_or_else(|| {
                let at = groups.len();
                groups.push((selector).clone());
                at
            });
        let schema = [InputShape {
            kinds: resolver(selector.relation),
            tree: None,
        }];
        let predicate = if let Some(prepared) = predicates.get(group) {
            prepared.clone()
        } else {
            let prepared = Arc::new(super::super::prepare::view(
                Arc::clone(&semantic),
                &ViewRequest::PredicateMatches(selector.clone()),
                &schema,
            )?);
            predicates.push(prepared.clone());
            prepared
        };
        let column = first_literal(selector, resolver(selector.relation))
            .map_or(ColumnIndex(0), |(column, _)| column);
        let key = (selector.relation, column);
        let candidate = if let Some(plan) = postings.get(&key) {
            (plan).clone()
        } else {
            let plan = Arc::new(super::super::prepare::view(
                Arc::clone(&semantic),
                &ViewRequest::CandidatePostings {
                    column,
                    literals: literals.remove(&key).unwrap_or_default(),
                },
                &schema,
            )?);
            postings.insert(key, (plan).clone());
            plan
        };
        let distinct = Arc::new(super::super::prepare::view(
            Arc::clone(&semantic),
            &ViewRequest::DistinctRepresentatives {
                columns: selector.columns.clone(),
            },
            &schema,
        )?);
        let mut selector_equalities = HashMap::new();
        for comparison in &selector.identity.comparisons {
            if comparison.test != Test::Equal {
                continue;
            }
            for operand in [&comparison.left, &comparison.right] {
                let Operand::Column(column) = operand else {
                    continue;
                };
                let key = (selector.relation, *column);
                let plan = if let Some(plan) = equalities.get(&key) {
                    (plan).clone()
                } else {
                    let plan = Arc::new(super::super::prepare::view(
                        Arc::clone(&semantic),
                        &ViewRequest::Equality {
                            columns: vec![*column],
                        },
                        &schema,
                    )?);
                    equalities.insert(key, (plan).clone());
                    plan
                };
                selector_equalities.insert(*column, plan);
            }
        }
        prepared.by_selector.insert(
            pointer,
            Arc::new(PreparedSelectorViews {
                group,
                predicate,
                postings: candidate,
                distinct,
                equalities: selector_equalities,
            }),
        );
    }
    prepared.groups = groups.len();
    Ok(Arc::new(prepared))
}

pub(in crate::application::file_dag::derived) fn activate_program(
    cache: &mut CacheBackend,
    prepared: &crate::application::file_dag::derived::prepared::PreparedDerived,
    demanded: &[crate::domain::coverage::types::RuleIndex],
) {
    let mut selectors = Vec::new();
    for use_ in demanded
        .iter()
        .flat_map(|rule| prepared.selectors[prepared.selector_ranges[rule.0].clone()].iter())
    {
        if !cache.eligible.contains(&use_.selector.relation) {
            continue;
        }
        let bytes = (selectors.len() + 1).checked_mul(size_of::<Arc<Selector>>());
        if bytes
            .and_then(|bytes| cache.bytes.checked_add(bytes))
            .is_none_or(|bytes| bytes > super::super::indexes::non_graph_limit(cache))
        {
            break;
        }
        selectors.push(Arc::clone(&use_.selector));
    }
    if let Some(bytes) = selectors
        .capacity()
        .checked_mul(size_of::<Arc<Selector>>())
        .and_then(|bytes| cache.bytes.checked_add(bytes))
        .filter(|bytes| *bytes <= super::super::indexes::non_graph_limit(cache))
    {
        cache.bytes = bytes;
        cache.graph_counts.peak_bytes = cache.graph_counts.peak_bytes.max(bytes);
        cache.selectors = selectors;
        super::super::indexes::candidate_columns(cache);
    }
    install_views(cache, Arc::clone(&prepared.views));
}

/// Install one immutable request descriptor and prepare active repetition once/file.
pub(in crate::application::file_dag::derived) fn install_views(
    cache: &mut CacheBackend,
    views: Arc<PreparedSelectionViews>,
) {
    // Active request bookkeeping is mandatory working metadata, independent of
    // optional retained payload capacity. No rows or derived payload live here.
    cache.selector_repetition.clear();
    cache.selector_repetition_failed = false;
    if cache
        .selector_repetition
        .try_reserve(cache.selectors.len())
        .is_ok()
    {
        for selector in &cache.selectors {
            if let Some(prepared) = views.by_selector.get(&(Arc::as_ptr(selector) as usize)) {
                if prepared.group >= views.groups {
                    cache.selector_repetition.clear();
                    break;
                }
                let count = cache.selector_repetition.entry(prepared.group).or_insert(0);
                *count = count.saturating_add(1).min(2);
            }
        }
    } else {
        cache.selector_repetition_failed = true;
    }
    tracing::trace!(target:"ryo::diagnostic::lifecycle_memory",phase="selector_activation",request_groups=views.groups,active_groups=cache.selector_repetition.len(),working_metadata_bytes=cache.selector_repetition.capacity().saturating_mul(size_of::<(usize, u8)>()));
    cache.selection_views = Some(views);
    super::fanout::prepare_active(cache);
}
