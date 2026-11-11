//! The route accounting's records: what each of its values states, which a format then writes.
use super::types::{
    RouteCountRow, RouteOwnerId, RouteOwnership, RoutePlanId, RouteRuleName, RouteRuleRow,
    RouteTally,
};
use crate::application::file_dag::route_counts::ids::service::RouteLabel;
use crate::application::file_dag::route_counts::ids::service::RouteNodeIndex;
use crate::application::file_dag::route_counts::ids::service::RouteRuleIndex;
use crate::outbound::route_counts::interface::RouteCountsPort;
use crate::outbound::route_counts::types::{RouteDocument, RouteField, RouteValue};

/// A value that states itself as a route record, which a format then writes.
pub(super) trait Record {
    /// The value as a route record.
    fn record(&self) -> RouteValue;
}
/// The document `format` writes for `value`.
pub(super) fn document(
    format: &dyn RouteCountsPort,
    value: &(impl Record + ?Sized),
) -> RouteDocument {
    format.document(&value.record())
}
/// One named field of a record.
pub(super) fn field(key: &'static str, value: &(impl Record + ?Sized)) -> RouteField {
    RouteField {
        key,
        value: value.record(),
    }
}

use super::constants::MAX_KEYS;
use super::ids::service::RouteFileIndex;
#[cfg(feature = "diagnostic-row-progress")]
use super::profile::service::CapturedOrigin;
use super::profile::service::with_state;
use super::profile::service::{FileScope, OriginScope, RouteProfile};
#[cfg(feature = "diagnostic-row-progress")]
use super::types::RouteProgressState;
use super::types::{
    BuildRoute, RouteActivity, RouteCount, RouteKey, RouteOrigin, RouteOutcome, RoutePopulations,
    RouteState,
};
use ryo_values::domain::runs::types::RuleId;
use std::collections::HashMap;

impl BuildRoute {
    pub(crate) fn label(self) -> RouteLabel {
        RouteLabel(match self {
            Self::OwnerAcquire => super::constants::OWNER_BUILD,
            Self::ClosurePartition => super::constants::CLOSURE_PARTITION_BUILD,
            Self::GraphAdjacency => super::constants::GRAPH_ADJACENCY_BUILD,
            Self::GraphDomain => super::constants::GRAPH_DOMAIN_BUILD,
        })
    }
}

pub(super) fn resource_routes_activity() -> RouteActivity {
    if tracing::enabled!(target: super::constants::RESOURCE_ROUTES_TARGET, tracing::Level::TRACE) {
        RouteActivity::Active
    } else {
        RouteActivity::Inactive
    }
}

#[cfg(feature = "diagnostic-row-progress")]
pub(super) fn new_progress() -> RouteProgressState {
    let now = std::time::Instant::now();
    RouteProgressState {
        started: now,
        last: now,
        ticks: RouteTally::default(),
        name: super::types::RouteFileName::default(),
        nodes: HashMap::new(),
        row_file: None,
        inputs: HashMap::new(),
    }
}

impl FileScope {
    /// Opens the profile of `file`, whose documents `format` writes; it records only when the route trace is on and a
    /// format is given.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_store::application::file_dag::route_counts::profile::service::FileScope;
    /// let scope = FileScope::begin(7_usize, None);
    /// let _profile = scope.profile();
    /// ```
    pub fn begin(
        file: impl Into<RouteFileIndex>,
        format: Option<std::sync::Arc<dyn RouteCountsPort>>,
    ) -> Self {
        let file = file.into();
        let enabled = resource_routes_activity();
        #[cfg(feature = "diagnostic-row-progress")]
        let enabled = if enabled == RouteActivity::Active
            || tracing::enabled!(target: super::constants::ROW_PROGRESS_TARGET, tracing::Level::TRACE)
        {
            RouteActivity::Active
        } else {
            RouteActivity::Inactive
        };
        let profile = if let (RouteActivity::Active, Some(format)) = (enabled, format) {
            RouteProfile(Some(std::sync::Arc::new(std::sync::Mutex::new(
                RouteState {
                    format,
                    file,
                    origin: RouteOrigin::default(),
                    counts: HashMap::new(),
                    overflow: RouteTally::default(),
                    rules: HashMap::new(),
                    #[cfg(feature = "diagnostic-row-progress")]
                    progress: new_progress(),
                },
            ))))
        } else {
            RouteProfile::default()
        };
        Self { profile }
    }
    /// The profile this scope opened, for the parts of the file to record into.
    pub fn profile(&self) -> &RouteProfile {
        &self.profile
    }
}

impl Drop for FileScope {
    fn drop(&mut self) {
        if self.profile.0.is_none() {
            return;
        }
        #[cfg(feature = "diagnostic-row-progress")]
        super::progress::service::flush(
            &self.profile,
            RouteLabel(super::constants::STAGE_FILE_END),
        );
        let Some((format, file, overflow, counts, names)) = with_state(&self.profile, |state| {
            (
                std::sync::Arc::clone(&state.format),
                state.file,
                state.overflow,
                std::mem::take(&mut state.counts),
                std::mem::take(&mut state.rules),
            )
        }) else {
            return;
        };
        if resource_routes_activity() == RouteActivity::Inactive {
            return;
        }
        let mut rows = counts.into_iter().collect::<Vec<_>>();
        rows.sort_by_key(|(key, _)| *key);
        let counts = document(
            &*format,
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
                        .unwrap_or(RouteLabel(super::constants::UNCLASSIFIED)),
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
        let mut names = names.into_iter().collect::<Vec<_>>();
        names.sort_by_key(|(index, _)| *index);
        let rules = document(
            &*format,
            &names
                .into_iter()
                .map(|(index, name)| RouteRuleRow { index, name })
                .collect::<Vec<_>>(),
        );
        tracing::trace!(target: super::constants::RESOURCE_ROUTES_TARGET, event = super::constants::RESOURCE_ROUTES_EVENT, file = file.0, overflow = overflow.0, counts = %counts, rules = %rules);
    }
}

#[cfg(feature = "diagnostic-row-progress")]
pub(crate) fn capture(profile: &RouteProfile) -> CapturedOrigin {
    CapturedOrigin {
        profile: profile.clone(),
        origin: with_state(profile, |state| state.origin),
    }
}

#[cfg(feature = "diagnostic-row-progress")]
impl CapturedOrigin {
    /// The profile this origin was captured from.
    pub fn profile(&self) -> &RouteProfile {
        &self.profile
    }
    /// Records under the captured origin again until the returned scope ends.
    pub fn resume(&self) -> OriginScope {
        scope(&self.profile, |current| {
            if let Some(origin) = self.origin {
                *current = origin;
            }
        })
    }
}

pub(crate) fn scope(profile: &RouteProfile, change: impl FnOnce(&mut RouteOrigin)) -> OriginScope {
    let previous = with_state(profile, |state| {
        let old = state.origin;
        change(&mut state.origin);
        old
    });
    OriginScope {
        profile: if previous.is_some() {
            profile.clone()
        } else {
            RouteProfile::default()
        },
        previous,
    }
}

impl RouteProfile {
    /// Records under `rule` until the returned scope ends.
    pub fn rule(&self, rule: impl Into<RouteRuleIndex>, name: &RuleId) -> OriginScope {
        let rule = rule.into();
        let _named = with_state(self, |state| {
            if !state.rules.contains_key(&rule) {
                if state.rules.len() < MAX_KEYS {
                    let _previous = state.rules.insert(rule, RouteRuleName(name.0.to_string()));
                } else {
                    state.overflow.0 += 1;
                }
            }
        });
        scope(self, |origin| {
            origin.rule = Some(rule);
            origin.node = None;
        })
    }
    /// Records under the program node `node` until the returned scope ends.
    pub fn node(&self, node: impl Into<RouteNodeIndex>) -> OriginScope {
        let node = node.into();
        scope(self, |origin| {
            origin.node = Some(node);
            #[cfg(feature = "diagnostic-row-progress")]
            {
                origin.namespace = RouteLabel(super::constants::DAG_NAMESPACE);
            }
        })
    }
    /// Records under the consumer `consumer` until the returned scope ends.
    pub fn consumer(&self, consumer: impl Into<RouteLabel>) -> OriginScope {
        let consumer = consumer.into();
        scope(self, |origin| origin.consumer = Some(consumer))
    }
}

pub(crate) fn identity(
    profile: &RouteProfile,
    plan: impl Into<RoutePlanId>,
    populations: impl Into<RoutePopulations>,
) -> OriginScope {
    let plan = plan.into();
    let populations = populations.into();
    scope(profile, |origin| {
        origin.plan = Some(plan);
        origin.populations = populations.0;
    })
}

pub(crate) fn owner(profile: &RouteProfile, owner_id: impl Into<RouteOwnerId>) -> OriginScope {
    let owner_id = owner_id.into();
    scope(profile, |origin| {
        origin.owner = RouteOwnership::Owned;
        origin.owner_id = Some(owner_id);
    })
}

impl Drop for OriginScope {
    fn drop(&mut self) {
        if let Some(old) = self.previous {
            let _restored = with_state(&self.profile, |state| state.origin = old);
        }
    }
}

pub(super) fn update(
    profile: &RouteProfile,
    route: RouteLabel,
    family: RouteLabel,
    outcome: RouteOutcome,
) {
    let _counted = with_state(profile, |state| {
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

pub(crate) fn event(
    profile: &RouteProfile,
    route: impl Into<RouteLabel>,
    family: impl Into<RouteLabel>,
) {
    update(profile, route.into(), family.into(), RouteOutcome::Called);
}

pub(crate) fn observe<T, E>(
    profile: &RouteProfile,
    route: impl Into<RouteLabel>,
    family: impl Into<RouteLabel>,
    run: impl FnOnce() -> Result<T, E>,
) -> Result<T, E> {
    let route = route.into();
    let family = family.into();
    update(profile, route, family, RouteOutcome::Called);
    let result = run();
    update(
        profile,
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

pub(crate) fn derived(kind: ryo_derived::domain::request::types::DerivedKind) -> RouteLabel {
    use ryo_derived::domain::request::types::DerivedKind::*;
    RouteLabel(match kind {
        Equality => super::constants::KIND_EQUALITY,
        Adjacency => super::constants::KIND_ADJACENCY,
        Membership => super::constants::KIND_MEMBERSHIP,
        PartitionOrder => super::constants::KIND_PARTITION_ORDER,
        ClosureAdjacency => super::constants::KIND_CLOSURE_ADJACENCY,
        CycleAdjacency => super::constants::KIND_CYCLE_ADJACENCY,
        OrderedKeys => super::constants::KIND_ORDERED_KEYS,
        OrderedDfs => super::constants::KIND_ORDERED_DFS,
        NativeLookup => super::constants::KIND_NATIVE_LOOKUP,
    })
}

pub(crate) fn representation(
    kind: ryo_execution_domain::domain::table_model::types::RepresentationKind,
) -> RouteLabel {
    use ryo_execution_domain::domain::table_model::types::RepresentationKind::*;
    RouteLabel(match kind {
        Stored => super::constants::REPRESENTATION_STORED,
        Tree => super::constants::REPRESENTATION_TREE,
        Computed => super::constants::REPRESENTATION_COMPUTED,
        Compact => super::constants::REPRESENTATION_COMPACT,
    })
}
