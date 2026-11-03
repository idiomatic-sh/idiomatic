//! Request preparation and bounded activation; never rebuild descriptors in a row path.
use super::*;
use crate::domain::{
    derived_views::collections::views::registration::registered,
    operation_schema::types::InputSchema, operator::contract::types::RequestOperators,
};
#[cfg(test)]
use ryo_language::domain::relations::service::kinds;
use ryo_language::domain::relations::types::Relation;
use std::collections::HashMap;

#[cfg(test)]
pub(crate) fn prepare_views(
    selectors: impl IntoIterator<Item = Arc<Selector>>,
) -> Result<Arc<PreparedSelectionViews>, OperatingError> {
    prepare_views_with_schemas(selectors, &kinds)
}

pub(crate) fn prepare_views_with_schemas<'s>(
    selectors: impl IntoIterator<Item = Arc<Selector>>,
    resolver: &dyn Fn(Relation) -> &'s [CellKind],
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
        let schema = [InputSchema {
            kinds: resolver(selector.relation),
            tree: None,
        }];
        let predicate = if let Some(prepared) = predicates.get(group) {
            prepared.clone()
        } else {
            let prepared = Arc::new(
                registered(DerivedKind::PredicateMatches)
                    .prepare(&ViewRequest::PredicateMatches(selector.clone()), &schema)?,
            );
            predicates.push(prepared.clone());
            prepared
        };
        let column = first_literal(selector, resolver(selector.relation))
            .map_or(ColumnIndex(0), |(column, _)| column);
        let key = (selector.relation, column);
        let candidate = if let Some(plan) = postings.get(&key) {
            (plan).clone()
        } else {
            let plan = Arc::new(registered(DerivedKind::CandidatePostings).prepare(
                &ViewRequest::CandidatePostings {
                    column,
                    literals: literals.remove(&key).unwrap_or_default(),
                },
                &schema,
            )?);
            postings.insert(key, (plan).clone());
            plan
        };
        let distinct = Arc::new(registered(DerivedKind::DistinctRepresentatives).prepare(
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
                    let plan = Arc::new(registered(DerivedKind::Equality).prepare(
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

pub(crate) fn prepare_request(pool: &mut PreparedPool, request: &RequestOperators) {
    if Arc::ptr_eq(&pool.selection_plans, &request.selection_plans)
        && pool
            .selection_views
            .as_ref()
            .is_some_and(|views| Arc::ptr_eq(views, &request.selection_views))
    {
        return;
    }
    pool.file_graph = None;
    pool.selection_plans = (request.selection_plans).clone();
    pool.selection_views = Some((request.selection_views).clone());
}

#[cfg(test)]
pub(crate) fn prepare(pool: &mut PreparedPool, plans: &[Plan], bindings: &[Vec<ParameterBinding>]) {
    if pool.selection_views.is_some()
        && pool.selection_plans.len() == plans.len()
        && plans
            .iter()
            .zip(bindings)
            .zip(pool.selection_plans.iter())
            .all(|((plan, bound), prepared)| {
                prepared.identity.0 == std::ptr::from_ref(plan).addr() && prepared.bound == *bound
            })
    {
        return;
    }
    let started = tracing::enabled!(target: "ryo::diagnostic::execution", tracing::Level::TRACE)
        .then(std::time::Instant::now);
    pool.file_graph = None;
    pool.selection_plans = Arc::new(
        plans
            .iter()
            .zip(bindings)
            .map(|(plan, bound)| SelectionProgram::new(plan, bound).prepare())
            .collect(),
    );
    // Fixed family dispatch can fail only if the closed registry is inconsistent;
    // authored schema errors remain deferred inside the prepared contracts.
    pool.selection_views = Some(
        prepare_views(
            pool.selection_plans
                .iter()
                .flat_map(|plan| plan.steps.iter().flatten())
                .cloned(),
        )
        .expect("registered selector families match their fixed requests"),
    );
    if let Some(started) = started {
        tracing::trace!(target: "ryo::diagnostic::execution", phase="stored_program_setup",elapsed_ns=started.elapsed().as_nanos() as u64,
            plans=pool.selection_plans.len(), selectors=pool.selection_plans.iter().map(|plan|plan.steps.iter().flatten().count()).sum::<usize>());
    }
}

pub(in crate::application::file_dag::derived) fn activate(
    cache: &mut CacheBackend,
    pool: &PreparedPool,
    selectable: &[Relation],
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
                .is_none_or(|bytes| bytes > super::super::indexes::non_graph_limit(cache))
            {
                break;
            }
            selectors.push((selector).clone());
        }
    }
    let bytes = selectors.capacity().checked_mul(size_of::<Arc<Selector>>());
    if let Some(bytes) = bytes
        .and_then(|bytes| cache.bytes.checked_add(bytes))
        .filter(|bytes| *bytes <= super::super::indexes::non_graph_limit(cache))
    {
        cache.bytes = bytes;
        cache.graph_counts.peak_bytes = cache.graph_counts.peak_bytes.max(bytes);
        cache.selectors = selectors;
        super::super::indexes::candidate_columns(cache);
    }
    if let Some(views) = &pool.selection_views {
        install_views(cache, (views).clone());
    }
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
        .try_reserve_exact(views.groups)
        .is_ok()
    {
        cache.selector_repetition.resize(views.groups, 0);
        for selector in &cache.selectors {
            if let Some(prepared) = views.by_selector.get(&(Arc::as_ptr(selector) as usize)) {
                let Some(count) = cache.selector_repetition.get_mut(prepared.group) else {
                    cache.selector_repetition.clear();
                    break;
                };
                *count = count.saturating_add(1).min(2);
            }
        }
    } else {
        cache.selector_repetition_failed = true;
    }
    tracing::trace!(target:"ryo::diagnostic::lifecycle_memory",phase="selector_activation",request_groups=views.groups,working_metadata_bytes=cache.selector_repetition.capacity());
    cache.selection_views = Some(views);
}
