//! Request preparation and bounded activation; never rebuild descriptors in a row path.
use crate::application::file_dag::derived::constants;
use crate::application::file_dag::derived::constants as activation;
use crate::application::file_dag::derived::owner::cache::types::CacheBackend;
use crate::application::file_dag::derived::owner::selection::literals::service::first_literal;
use crate::application::file_dag::derived::owner::selection::types::*;
use crate::domain::{
    derived_views::types::{DerivedContract, ViewRequest},
    operating::errors::OperatingError,
    stored_selection::types::{PreparedSelectionViews, PreparedSelectorViews, Selector},
};
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::types::{ColumnIndex, LiteralText, Operand, Test};
use std::sync::Arc;

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
        if let Some(FirstLiteral { column, text }) =
            first_literal(selector, resolver(selector.relation))
        {
            literals
                .entry((selector.relation, column))
                .or_default()
                .push(LiteralText(text.0.into()));
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
            .position(|previous| same(previous, selector) == SelectorMatch::Same)
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
            let prepared = Arc::new(crate::application::file_dag::derived::prepare::view(
                Arc::clone(&semantic),
                &ViewRequest::PredicateMatches(selector.clone()),
                &schema,
            )?);
            predicates.push(prepared.clone());
            prepared
        };
        let column = first_literal(selector, resolver(selector.relation))
            .map_or(ColumnIndex(0), |literal| literal.column);
        let key = (selector.relation, column);
        let candidate = if let Some(plan) = postings.get(&key) {
            (plan).clone()
        } else {
            let plan = Arc::new(crate::application::file_dag::derived::prepare::view(
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
        let distinct = Arc::new(crate::application::file_dag::derived::prepare::view(
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
                    let plan = Arc::new(crate::application::file_dag::derived::prepare::view(
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

pub(in crate::application::file_dag::derived::owner) fn activate_program(
    cache: &mut CacheBackend,
    prepared: &crate::application::file_dag::derived::prepared::RequestDerived,
    demanded: &[crate::domain::coverage::types::RuleIndex],
) -> Result<(), OperatingError> {
    let mut selectors = Vec::new();
    'active: for rule in demanded {
        for use_ in prepared.selectors_for(*rule)? {
            tracing::trace!(target: activation::TARGET, event = activation::SELECTOR);
            if !cache.eligible.contains(&use_.selector.relation) {
                continue;
            }
            let bytes = (selectors.len() + 1).checked_mul(size_of::<Arc<Selector>>());
            if bytes
                .and_then(|bytes| cache.bytes.checked_add(bytes))
                .is_none_or(|bytes| {
                    bytes
                    > crate::application::file_dag::derived::owner::index::service::non_graph_limit(
                        cache,
                    )
                    .0
                })
            {
                break 'active;
            }
            selectors.push(Arc::clone(&use_.selector));
        }
    }
    if let Some(bytes) = crate::application::file_dag::derived::ledger::capacity::checked::<
        Arc<Selector>,
    >(
        crate::application::file_dag::derived::ledger::types::SlotCount(selectors.capacity()),
    )
    .and_then(|bytes| cache.bytes.checked_add(bytes.0))
    .filter(|bytes| {
        *bytes
            <= crate::application::file_dag::derived::owner::index::service::non_graph_limit(cache)
                .0
    }) {
        cache.bytes = bytes;
        cache.graph_counts.peak_bytes = cache.graph_counts.peak_bytes.max(bytes);
        cache.selectors = selectors;
        crate::application::file_dag::derived::owner::index::service::candidate_columns(cache);
    }
    let views = cache
        .selection_views
        .as_ref()
        .cloned()
        .ok_or(OperatingError::NotRun)?;
    install_views(cache, views);
    Ok(())
}

/// Install one immutable request descriptor and prepare active repetition once/file.
pub(in crate::application::file_dag::derived::owner) fn install_views(
    cache: &mut CacheBackend,
    views: Arc<PreparedSelectionViews>,
) {
    // Active request bookkeeping is mandatory working metadata, independent of
    // optional retained payload capacity. No rows or derived payload live here.
    cache.selector_repetition.clear();
    cache.selector_repetition_failed =
        crate::application::file_dag::derived::owner::cache::types::RepetitionTracking::Tracked;
    if cache
        .selector_repetition
        .try_reserve(cache.selectors.len())
        .is_ok()
    {
        for selector in &cache.selectors {
            tracing::trace!(target: activation::TARGET, event = activation::REPETITION);
            if let Some(prepared) = views.by_selector.get(&(Arc::as_ptr(selector) as usize)) {
                if prepared.group >= views.groups {
                    cache.selector_repetition.clear();
                    break;
                }
                let count = cache
                    .selector_repetition
                    .entry(
                        crate::application::file_dag::derived::owner::cache::types::RepetitionGroup(
                            prepared.group,
                        ),
                    )
                    .or_default();
                count.0 = count.0.saturating_add(1).min(2);
            }
        }
    } else {
        cache.selector_repetition_failed =
            crate::application::file_dag::derived::owner::cache::types::RepetitionTracking::Failed;
    }
    tracing::trace!(target:constants::TARGET_LIFECYCLE_MEMORY,phase=constants::PHASE_SELECTOR_ACTIVATION,request_groups=views.groups,active_groups=cache.selector_repetition.len(),working_metadata_bytes=cache.selector_repetition.capacity().saturating_mul(size_of::<(
            crate::application::file_dag::derived::owner::cache::types::RepetitionGroup,
            crate::application::file_dag::derived::owner::cache::types::Repetitions,
        )>()));
    cache.selection_views = Some(views);
    super::super::fanout::service::prepare_active(cache);
}

/// Prepare request-specific metadata once per worker, without evaluating any relation.
/// Exact ordered predicates and referenced binding values; origins/projections are not result identity.
fn same(left: &Selector, right: &Selector) -> SelectorMatch {
    if left.relation == right.relation
        && left.identity.comparisons == right.identity.comparisons
        && left.identity.bindings == right.identity.bindings
    {
        SelectorMatch::Same
    } else {
        SelectorMatch::Different
    }
}
