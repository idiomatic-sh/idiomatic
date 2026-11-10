//! Diagnostic vocabulary for actual derived activation boundaries and visits.
/// The tracing target of the derived activation events.
pub const TARGET: &str = "ryo::diagnostic::activation";
pub(super) const BEGIN: &str = "begin";
pub(super) const END: &str = "end";
pub(super) const RELATION_RULE: &str = "relation_rule";
pub(super) const RELATION: &str = "relation_visit";
pub(super) const GRAPH_RULE: &str = "graph_rule";
pub(super) const GRAPH_HIT: &str = "graph_hit";
/// Every read of one rule's private prepared metadata, whatever phase asks.
pub const RULE_METADATA: &str = "rule_metadata";
/// Diagnostic and expectation vocabulary of the derived owner.
pub(crate) const TARGET_GRAPH: &str = "ryo::diagnostic::graph";
/// The tracing target of the resource-route counts.
pub const TARGET_RESOURCE_ROUTES: &str =
    ryo_values::domain::table_words::constants::RESOURCE_ROUTES_TARGET;
pub(crate) const TARGET_STORED_JOIN: &str = "ryo::diagnostic::stored_join";
/// The tracing target of the lifecycle memory events.
pub const TARGET_LIFECYCLE_MEMORY: &str =
    ryo_values::domain::table_words::constants::LIFECYCLE_TARGET;
pub(crate) const EVENT_OWNER_FOOTPRINT: &str = "owner_footprint";
pub(crate) const EVENT_FILE_GRAPHS: &str = "file_graphs";
pub(crate) const EVENT_FILE_INDEXES: &str = "file_indexes";
pub(crate) const EVENT_ADJACENCY: &str = "adjacency";
pub(crate) const EVENT_DOMAIN: &str = "graph_domain";
pub(crate) const ROUTE_GRAPH: &str = "graph";
pub(crate) const REACH_REQUEST: &str = "reach_request";
pub(crate) const REACH_MEMO_HIT: &str = "reach_memo_hit";
pub(crate) const REACH_TRAVERSAL: &str = "reach_traversal";
pub(crate) const NEAREST_REQUEST: &str = "nearest_request";
pub(crate) const NEAREST_TRAVERSAL: &str = "nearest_traversal";
pub(crate) const NEAREST_MEMO_HIT: &str = "nearest_memo_hit";
pub(crate) const OWNER_REQUEST: &str = "owner_request";
pub(crate) const OWNER_HIT: &str = "owner_hit";
#[cfg(test)]
pub(crate) const EXPECT_GRAPH_LEASE: &str = "held graph has its required backing lease";
#[cfg(test)]
pub(crate) const EXPECT_MEMBERSHIP_LEASE: &str = "held membership has its required backing lease";
/// Names of the admitted acquisition origin in its debug form.
pub(super) const ORIGIN_TYPE: &str = "AcquisitionOrigin";
pub(super) const ORIGIN_LIFETIME: &str = "lifetime";
pub(super) const ORIGIN_KIND: &str = "kind";
