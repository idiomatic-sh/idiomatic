//! Values of the opt-in route accounting; never participates in execution or admission decisions.
use std::{collections::HashMap, fmt};

/// Text label of one route, family, counter, namespace or consumer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct RouteLabel(pub &'static str);
impl From<&'static str> for RouteLabel {
    fn from(text: &'static str) -> Self {
        Self(text)
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct RouteRuleIndex(pub usize);
impl From<usize> for RouteRuleIndex {
    fn from(index: usize) -> Self {
        Self(index)
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct RouteNodeIndex(pub usize);
impl From<usize> for RouteNodeIndex {
    fn from(index: usize) -> Self {
        Self(index)
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct RouteOwnerId(pub usize);
impl From<usize> for RouteOwnerId {
    fn from(id: usize) -> Self {
        Self(id)
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct RoutePlanId(pub usize);
impl From<usize> for RoutePlanId {
    fn from(id: usize) -> Self {
        Self(id)
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct RoutePopulationId(pub usize);
/// The two live population identities of one derived request.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct RoutePopulations(pub [Option<RoutePopulationId>; 2]);
impl From<[Option<usize>; 2]> for RoutePopulations {
    fn from(ids: [Option<usize>; 2]) -> Self {
        Self(ids.map(|id| id.map(RoutePopulationId)))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RouteFileIndex(pub usize);
impl From<usize> for RouteFileIndex {
    fn from(index: usize) -> Self {
        Self(index)
    }
}
/// Number of events; counts and overflow are summed in place.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct RouteTally(pub u64);

/// Name of a rule, owned once per file profile.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct RouteRuleName(pub String);
/// A JSON document carried as one tracing field.
#[derive(Clone, Debug, Default)]
pub(crate) struct RouteDocument(pub String);
impl fmt::Display for RouteDocument {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.write_str(&self.0)
    }
}

/// Whether the build site belongs to an owner scope.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum RouteOwnership {
    #[default]
    Unowned,
    Owned,
}
/// Whether a file scope records into the thread profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RouteActivity {
    Active,
    Inactive,
}
/// Which counter of a route event moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RouteOutcome {
    Called,
    Succeeded,
    Failed,
}

/// Explicit caller route survives delegation; only the physical build site increments it.
#[derive(Clone, Copy, Debug)]
pub(crate) enum BuildRoute {
    OwnerAcquire,
    SelectorCandidate,
    Column,
    SelectorPredicate,
    SelectorPostings,
    SelectorDistinct,
    ClosurePartition,
    GraphAdjacency,
    GraphDomain,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct RouteOrigin {
    pub rule: Option<RouteRuleIndex>,
    pub node: Option<RouteNodeIndex>,
    pub owner: RouteOwnership,
    pub owner_id: Option<RouteOwnerId>,
    pub consumer: Option<RouteLabel>,
    pub plan: Option<RoutePlanId>,
    pub populations: [Option<RoutePopulationId>; 2],
    #[cfg(feature = "diagnostic-row-progress")]
    pub namespace: RouteLabel,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct RouteKey {
    pub origin: RouteOrigin,
    pub route: RouteLabel,
    pub family: RouteLabel,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct RouteCount {
    pub calls: RouteTally,
    pub successes: RouteTally,
    pub failures: RouteTally,
}
pub(crate) struct RouteState {
    pub file: RouteFileIndex,
    pub origin: RouteOrigin,
    pub counts: HashMap<RouteKey, RouteCount>,
    pub overflow: RouteTally,
    pub rules: HashMap<RouteRuleIndex, RouteRuleName>,
    #[cfg(feature = "diagnostic-row-progress")]
    pub progress: RouteProgressState,
}

pub(crate) struct FileScope {
    pub active: RouteActivity,
}
pub(crate) struct OriginScope {
    pub previous: Option<RouteOrigin>,
}

#[cfg(feature = "diagnostic-row-progress")]
#[derive(Clone, Copy)]
pub(crate) struct CapturedOrigin(pub Option<RouteOrigin>);
/// Display path of the file under profile.
#[cfg(feature = "diagnostic-row-progress")]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct RouteFileName(pub String);
#[cfg(feature = "diagnostic-row-progress")]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct RouteOperationName(pub String);
#[cfg(feature = "diagnostic-row-progress")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct RouteCardinality(pub usize);
#[cfg(feature = "diagnostic-row-progress")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct RouteInputSlot(pub usize);
#[cfg(feature = "diagnostic-row-progress")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct RouteNodeKey {
    pub rule: Option<RouteRuleIndex>,
    pub namespace: RouteLabel,
    pub node: RouteNodeIndex,
}
#[cfg(feature = "diagnostic-row-progress")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct RouteInputKey {
    pub rule: Option<RouteRuleIndex>,
    pub namespace: RouteLabel,
    pub node: Option<RouteNodeIndex>,
    pub slot: RouteInputSlot,
}
#[cfg(feature = "diagnostic-row-progress")]
#[derive(Clone, Debug)]
pub(crate) struct RouteRowFile {
    pub file: ryo_relations::domain::relations::types::FileId,
    pub path: RouteFileName,
}
#[cfg(feature = "diagnostic-row-progress")]
pub(crate) struct RouteProgressState {
    pub started: std::time::Instant,
    pub last: std::time::Instant,
    pub ticks: RouteTally,
    pub name: RouteFileName,
    pub nodes: HashMap<RouteNodeKey, RouteOperationName>,
    pub row_file: Option<RouteRowFile>,
    pub inputs: HashMap<RouteInputKey, RouteCardinality>,
}
/// Everything one progress event reports, read under one borrow of the profile.
#[cfg(feature = "diagnostic-row-progress")]
pub(crate) struct RouteSnapshot {
    pub file: RouteFileIndex,
    pub path: RouteFileName,
    pub origin: RouteOrigin,
    pub rule: RouteRuleName,
    pub elapsed_ms: RouteTally,
    pub ticks: RouteTally,
    pub overflow: RouteTally,
    pub hottest: RouteDocument,
    pub counters: RouteDocument,
    pub operation: RouteOperationName,
    pub input_cardinalities: [Option<RouteCardinality>; 3],
    pub row_file: Option<RouteRowFile>,
}

pub(crate) struct RouteCountRow {
    pub rule: Option<RouteRuleIndex>,
    pub node: Option<RouteNodeIndex>,
    pub owner: RouteOwnership,
    pub owner_id: Option<RouteOwnerId>,
    pub consumer: RouteLabel,
    pub plan: Option<RoutePlanId>,
    pub populations: [Option<RoutePopulationId>; 2],
    pub route: RouteLabel,
    pub family: RouteLabel,
    pub calls: RouteTally,
    pub successes: RouteTally,
    pub failures: RouteTally,
}

pub(crate) struct RouteRuleRow {
    pub index: RouteRuleIndex,
    pub name: RouteRuleName,
}

#[cfg(feature = "diagnostic-row-progress")]
pub(crate) struct RouteHotRow {
    pub rule: Option<RouteRuleIndex>,
    pub node: Option<RouteNodeIndex>,
    pub counter: RouteLabel,
    pub count: RouteTally,
    pub rule_name: RouteRuleName,
    pub input_cardinalities: [Option<RouteCardinality>; 3],
    pub node_namespace: RouteLabel,
}

#[cfg(feature = "diagnostic-row-progress")]
#[derive(Default)]
pub(crate) struct RouteCounters {
    pub input_rows: RouteTally,
    pub emitted_rows: RouteTally,
    pub predicate_evaluations: RouteTally,
    pub index_probes: RouteTally,
    pub index_chain_rows_examined: RouteTally,
    pub equality_matches: RouteTally,
    pub join_candidate_pairs: RouteTally,
    pub kernel_output_rows: RouteTally,
    pub position_rows_yielded: RouteTally,
}
