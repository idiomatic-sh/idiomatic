//! Diagnostic vocabulary for actual derived activation boundaries and visits.
pub(in crate::application::file_dag) const TARGET: &str = "ryo::diagnostic::activation";
pub(super) const BEGIN: &str = "begin";
pub(super) const END: &str = "end";
pub(super) const RELATION_RULE: &str = "relation_rule";
pub(super) const RELATION: &str = "relation_visit";
pub(super) const GRAPH_RULE: &str = "graph_rule";
pub(super) const GRAPH_HIT: &str = "graph_hit";
/// Every read of one rule's private prepared metadata, whatever phase asks.
pub(in crate::application::file_dag) const RULE_METADATA: &str = "rule_metadata";
/// Diagnostic and expectation vocabulary of the derived owner.
pub(in crate::application::file_dag) const TARGET_GRAPH: &str = "ryo::diagnostic::graph";
pub(in crate::application::file_dag) const TARGET_RESOURCE_ROUTES: &str =
    "ryo::diagnostic::resource_routes";
pub(in crate::application::file_dag) const TARGET_STORED_JOIN: &str =
    "ryo::diagnostic::stored_join";
pub(in crate::application::file_dag) const TARGET_LIFECYCLE_MEMORY: &str =
    ryo_values::domain::table_words::constants::LIFECYCLE_TARGET;
pub(in crate::application::file_dag) const EVENT_OWNER_FOOTPRINT: &str = "owner_footprint";
pub(in crate::application::file_dag) const EVENT_FILE_GRAPHS: &str = "file_graphs";
pub(in crate::application::file_dag) const EVENT_FILE_INDEXES: &str = "file_indexes";
pub(in crate::application::file_dag) const EVENT_ADJACENCY: &str = "adjacency";
pub(in crate::application::file_dag) const EVENT_DOMAIN: &str = "graph_domain";
pub(in crate::application::file_dag) const ROUTE_GRAPH: &str = "graph";
pub(in crate::application::file_dag) const REACH_REQUEST: &str = "reach_request";
pub(in crate::application::file_dag) const REACH_MEMO_HIT: &str = "reach_memo_hit";
pub(in crate::application::file_dag) const REACH_TRAVERSAL: &str = "reach_traversal";
pub(in crate::application::file_dag) const NEAREST_REQUEST: &str = "nearest_request";
pub(in crate::application::file_dag) const NEAREST_TRAVERSAL: &str = "nearest_traversal";
pub(in crate::application::file_dag) const NEAREST_MEMO_HIT: &str = "nearest_memo_hit";
pub(in crate::application::file_dag) const OWNER_REQUEST: &str = "owner_request";
pub(in crate::application::file_dag) const OWNER_HIT: &str = "owner_hit";
pub(in crate::application::file_dag) const EXPECT_REVERSE_LIST: &str = "installed reverse list";
pub(in crate::application::file_dag) const EXPECT_KNOWN_POPULATION: &str = "known population";
pub(in crate::application::file_dag) const EXPECT_EXISTING_VIEW: &str = "existing view";
#[cfg(test)]
pub(in crate::application::file_dag) const EXPECT_GRAPH_LEASE: &str =
    "held graph has its required backing lease";
#[cfg(test)]
pub(in crate::application::file_dag) const EXPECT_MEMBERSHIP_LEASE: &str =
    "held membership has its required backing lease";
pub(in crate::application::file_dag) const EXPECT_QUALIFYING_ENCOUNTER: &str =
    "a qualifying encounter";
/// Names of the admitted acquisition origin in its debug form.
pub(super) const ORIGIN_TYPE: &str = "AcquisitionOrigin";
pub(super) const ORIGIN_LIFETIME: &str = "lifetime";
pub(super) const ORIGIN_KIND: &str = "kind";
