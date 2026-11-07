//! Opt-in diagnostic accounting; never participates in execution or admission decisions.
mod constants;
pub mod profile;
#[cfg(feature = "diagnostic-row-progress")]
pub(crate) mod progress;
mod schemas;
#[cfg(test)]
mod test;
pub(crate) mod types;

use std::collections::HashMap;

use ryo_values::domain::runs::types::RuleId;

use constants::MAX_KEYS;
#[cfg(feature = "diagnostic-row-progress")]
use profile::service::with_borrow;
use profile::service::with_borrow_mut;
use schemas::document;
use types::{
    BuildRoute, FileScope, OriginScope, RouteActivity, RouteCount, RouteCountRow, RouteFileIndex,
    RouteKey, RouteLabel, RouteNodeIndex, RouteOrigin, RouteOutcome, RouteOwnerId, RouteOwnership,
    RoutePlanId, RoutePopulations, RouteRuleIndex, RouteRuleName, RouteRuleRow, RouteState,
    RouteTally,
};
#[cfg(feature = "diagnostic-row-progress")]
use types::{CapturedOrigin, RouteProgressState};

impl BuildRoute {
    pub(crate) fn label(self) -> RouteLabel {
        RouteLabel(match self {
            Self::OwnerAcquire => constants::OWNER_BUILD,
            Self::SelectorCandidate => constants::SELECTOR_CANDIDATE_BUILD,
            Self::Column => constants::COLUMN_BUILD,
            Self::SelectorPredicate => constants::SELECTOR_PREDICATE_BUILD,
            Self::SelectorPostings => constants::SELECTOR_POSTINGS_BUILD,
            Self::SelectorDistinct => constants::SELECTOR_DISTINCT_BUILD,
            Self::ClosurePartition => constants::CLOSURE_PARTITION_BUILD,
            Self::GraphAdjacency => constants::GRAPH_ADJACENCY_BUILD,
            Self::GraphDomain => constants::GRAPH_DOMAIN_BUILD,
        })
    }
}

fn resource_routes_activity() -> RouteActivity {
    if tracing::enabled!(target: constants::RESOURCE_ROUTES_TARGET, tracing::Level::TRACE) {
        RouteActivity::Active
    } else {
        RouteActivity::Inactive
    }
}
#[cfg(feature = "diagnostic-row-progress")]
fn new_progress() -> RouteProgressState {
    let now = std::time::Instant::now();
    RouteProgressState {
        started: now,
        last: now,
        ticks: RouteTally::default(),
        name: types::RouteFileName::default(),
        nodes: HashMap::new(),
        row_file: None,
        inputs: HashMap::new(),
    }
}

impl FileScope {
    pub(crate) fn begin(file: impl Into<RouteFileIndex>) -> Self {
        let file = file.into();
        let enabled = resource_routes_activity();
        #[cfg(feature = "diagnostic-row-progress")]
        let enabled = if enabled == RouteActivity::Active
            || tracing::enabled!(target: constants::ROW_PROGRESS_TARGET, tracing::Level::TRACE)
        {
            RouteActivity::Active
        } else {
            RouteActivity::Inactive
        };
        let active = if enabled == RouteActivity::Active {
            with_borrow_mut(|state| {
                if let Some(parent) = state {
                    // A nested file cannot be attributed by this bounded synchronous profile.
                    // Mark the aggregate incomplete rather than changing execution behavior.
                    parent.overflow.0 += 1;
                    return RouteActivity::Inactive;
                }
                *state = Some(Box::new(RouteState {
                    file,
                    origin: RouteOrigin::default(),
                    counts: HashMap::new(),
                    overflow: RouteTally::default(),
                    rules: HashMap::new(),
                    #[cfg(feature = "diagnostic-row-progress")]
                    progress: new_progress(),
                }));
                RouteActivity::Active
            })
        } else {
            RouteActivity::Inactive
        };
        Self { active }
    }
}
impl Drop for FileScope {
    fn drop(&mut self) {
        if self.active == RouteActivity::Inactive {
            return;
        }
        #[cfg(feature = "diagnostic-row-progress")]
        progress::service::flush(RouteLabel(constants::STAGE_FILE_END));
        if let Some(state) = with_borrow_mut(Option::take) {
            if resource_routes_activity() == RouteActivity::Inactive {
                return;
            }
            let mut rows = state.counts.into_iter().collect::<Vec<_>>();
            rows.sort_by_key(|(key, _)| *key);
            let counts = document(
                &rows
                    .into_iter()
                    .map(|(key, count)| RouteCountRow {
                        rule: key.origin.rule,
                        node: key.origin.node,
                        owner: key.origin.owner,
                        owner_id: key.origin.owner_id,
                        consumer: key
                            .origin
                            .consumer
                            .unwrap_or(RouteLabel(constants::UNCLASSIFIED)),
                        plan: key.origin.plan,
                        populations: key.origin.populations,
                        route: key.route,
                        family: key.family,
                        calls: count.calls,
                        successes: count.successes,
                        failures: count.failures,
                    })
                    .collect::<Vec<_>>(),
            );
            let mut names = state.rules.into_iter().collect::<Vec<_>>();
            names.sort_by_key(|(index, _)| *index);
            let rules = document(
                &names
                    .into_iter()
                    .map(|(index, name)| RouteRuleRow { index, name })
                    .collect::<Vec<_>>(),
            );
            tracing::trace!(target: constants::RESOURCE_ROUTES_TARGET, event = constants::RESOURCE_ROUTES_EVENT, file = state.file.0, overflow = state.overflow.0, counts = %counts, rules = %rules);
        }
    }
}

#[cfg(feature = "diagnostic-row-progress")]
pub(crate) fn capture() -> CapturedOrigin {
    CapturedOrigin(with_borrow(|state| state.as_ref().map(|state| state.origin)))
}
#[cfg(feature = "diagnostic-row-progress")]
pub(crate) fn resume(origin: CapturedOrigin) -> OriginScope {
    scope(|current| {
        if let Some(origin) = origin.0 {
            *current = origin;
        }
    })
}
pub(super) fn scope(change: impl FnOnce(&mut RouteOrigin)) -> OriginScope {
    let previous = with_borrow_mut(|state| {
        state.as_mut().map(|state| {
            let old = state.origin;
            change(&mut state.origin);
            old
        })
    });
    OriginScope { previous }
}
pub(crate) fn rule(rule: impl Into<RouteRuleIndex>, name: &RuleId) -> OriginScope {
    let rule = rule.into();
    with_borrow_mut(|state| {
        if let Some(state) = state {
            if !state.rules.contains_key(&rule) {
                if state.rules.len() < MAX_KEYS {
                    state.rules.insert(rule, RouteRuleName(name.0.to_string()));
                } else {
                    state.overflow.0 += 1;
                }
            }
        }
    });
    scope(|origin| {
        origin.rule = Some(rule);
        origin.node = None;
    })
}
pub(crate) fn node(node: impl Into<RouteNodeIndex>) -> OriginScope {
    let node = node.into();
    scope(|origin| {
        origin.node = Some(node);
        #[cfg(feature = "diagnostic-row-progress")]
        {
            origin.namespace = RouteLabel(constants::DAG_NAMESPACE);
        }
    })
}
pub(crate) fn consumer(consumer: impl Into<RouteLabel>) -> OriginScope {
    let consumer = consumer.into();
    scope(|origin| origin.consumer = Some(consumer))
}
pub(crate) fn identity(
    plan: impl Into<RoutePlanId>,
    populations: impl Into<RoutePopulations>,
) -> OriginScope {
    let plan = plan.into();
    let populations = populations.into();
    scope(|origin| {
        origin.plan = Some(plan);
        origin.populations = populations.0;
    })
}
pub(crate) fn owner(owner_id: impl Into<RouteOwnerId>) -> OriginScope {
    let owner_id = owner_id.into();
    scope(|origin| {
        origin.owner = RouteOwnership::Owned;
        origin.owner_id = Some(owner_id);
    })
}
impl Drop for OriginScope {
    fn drop(&mut self) {
        if let Some(old) = self.previous {
            with_borrow_mut(|state| {
                if let Some(state) = state {
                    state.origin = old;
                }
            });
        }
    }
}
fn update(route: RouteLabel, family: RouteLabel, outcome: RouteOutcome) {
    with_borrow_mut(|state| {
        let Some(state) = state else {
            return;
        };
        let key = RouteKey {
            origin: state.origin,
            route,
            family,
        };
        if !state.counts.contains_key(&key) && state.counts.len() == MAX_KEYS {
            state.overflow.0 += 1;
            return;
        }
        let count: &mut RouteCount = state.counts.entry(key).or_default();
        match outcome {
            RouteOutcome::Called => count.calls.0 += 1,
            RouteOutcome::Succeeded => count.successes.0 += 1,
            RouteOutcome::Failed => count.failures.0 += 1,
        }
    });
}
pub(crate) fn event(route: impl Into<RouteLabel>, family: impl Into<RouteLabel>) {
    update(route.into(), family.into(), RouteOutcome::Called);
}
pub(crate) fn observe<T, E>(
    route: impl Into<RouteLabel>,
    family: impl Into<RouteLabel>,
    run: impl FnOnce() -> Result<T, E>,
) -> Result<T, E> {
    let route = route.into();
    let family = family.into();
    update(route, family, RouteOutcome::Called);
    let result = run();
    update(
        route,
        family,
        if result.is_ok() {
            RouteOutcome::Succeeded
        } else {
            RouteOutcome::Failed
        },
    );
    result
}
pub(crate) fn derived(kind: ryo_derived::domain::contract::types::DerivedKind) -> RouteLabel {
    use ryo_derived::domain::contract::types::DerivedKind::*;
    RouteLabel(match kind {
        Equality => constants::KIND_EQUALITY,
        PredicateMatches => constants::KIND_PREDICATE_MATCHES,
        CandidatePostings => constants::KIND_CANDIDATE_POSTINGS,
        DistinctRepresentatives => constants::KIND_DISTINCT_REPRESENTATIVES,
        Adjacency => constants::KIND_ADJACENCY,
        Membership => constants::KIND_MEMBERSHIP,
        PartitionOrder => constants::KIND_PARTITION_ORDER,
        ClosureAdjacency => constants::KIND_CLOSURE_ADJACENCY,
        CycleAdjacency => constants::KIND_CYCLE_ADJACENCY,
        OrderedKeys => constants::KIND_ORDERED_KEYS,
        OrderedDfs => constants::KIND_ORDERED_DFS,
        NativeLookup => constants::KIND_NATIVE_LOOKUP,
        RawKeyOrder => constants::KIND_RAW_KEY_ORDER,
    })
}
pub(crate) fn representation(
    kind: crate::domain::table_model::collections::representations::types::RepresentationKind,
) -> RouteLabel {
    use crate::domain::table_model::collections::representations::types::RepresentationKind::*;
    RouteLabel(match kind {
        Stored => constants::REPRESENTATION_STORED,
        Selected => constants::REPRESENTATION_SELECTED,
        Tree => constants::REPRESENTATION_TREE,
        Computed => constants::REPRESENTATION_COMPUTED,
        Compact => constants::REPRESENTATION_COMPACT,
        Positioned => constants::REPRESENTATION_POSITIONED,
    })
}
