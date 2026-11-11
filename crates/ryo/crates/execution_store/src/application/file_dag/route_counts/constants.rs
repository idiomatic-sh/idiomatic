//! Diagnostic vocabulary of the opt-in route accounting.
use crate::outbound::route_counts::types::RecordKey;
pub(super) const MAX_KEYS: usize = 32_768;
pub(super) const RESOURCE_ROUTES_TARGET: &str =
    ryo_values::domain::table_words::constants::RESOURCE_ROUTES_TARGET;
pub(super) const RESOURCE_ROUTES_EVENT: &str = "resource_routes";
pub(super) const UNCLASSIFIED: &str = "unclassified";

pub(super) const OWNER_BUILD: &str = "owner_build";
pub(super) const CLOSURE_PARTITION_BUILD: &str = "closure_partition_build";
pub(super) const GRAPH_ADJACENCY_BUILD: &str = "graph_adjacency_build";
pub(super) const GRAPH_DOMAIN_BUILD: &str = "graph_domain_build";

pub(super) const KIND_EQUALITY: &str = "Equality";
pub(super) const KIND_ADJACENCY: &str = "Adjacency";
pub(super) const KIND_MEMBERSHIP: &str = "Membership";
pub(super) const KIND_PARTITION_ORDER: &str = "PartitionOrder";
pub(super) const KIND_CLOSURE_ADJACENCY: &str = "ClosureAdjacency";
pub(super) const KIND_CYCLE_ADJACENCY: &str = "CycleAdjacency";
pub(super) const KIND_ORDERED_KEYS: &str = "OrderedKeys";
pub(super) const KIND_ORDERED_DFS: &str = "OrderedDfs";
pub(super) const KIND_NATIVE_LOOKUP: &str = "NativeLookup";

pub(super) const REPRESENTATION_STORED: &str = "Stored";
pub(super) const REPRESENTATION_TREE: &str = "Tree";
pub(super) const REPRESENTATION_COMPUTED: &str = "Computed";
pub(super) const REPRESENTATION_COMPACT: &str = "Compact";

#[cfg(feature = "diagnostic-row-progress")]
pub(super) const ROW_PROGRESS: &str = "row_progress";
#[cfg(feature = "diagnostic-row-progress")]
pub(super) const ROW_PROGRESS_TARGET: &str = "ryo::diagnostic::row_progress";
#[cfg(feature = "diagnostic-row-progress")]
pub(super) const DAG_NAMESPACE: &str = "dag";
#[cfg(feature = "diagnostic-row-progress")]
pub(super) const NO_RULE: &str = "<none>";
#[cfg(feature = "diagnostic-row-progress")]
pub(super) const PHASE_PROJECT: &str = ryo_values::domain::descriptions::constants::PROJECT;
#[cfg(feature = "diagnostic-row-progress")]
pub(super) const PHASE_FILE: &str = "file";
#[cfg(feature = "diagnostic-row-progress")]
pub(super) const STAGE_FILE_END: &str = "file_end";
#[cfg(feature = "diagnostic-row-progress")]
pub(super) const STAGE_SCOPE_START: &str = "scope_start";
#[cfg(feature = "diagnostic-row-progress")]
pub(super) const STAGE_PROGRESS: &str = "progress";
#[cfg(feature = "diagnostic-row-progress")]
pub(super) const HOTTEST_LIMIT: usize = 16;
#[cfg(feature = "diagnostic-row-progress")]
pub(super) const INPUT_SLOTS: usize = 3;
#[cfg(feature = "diagnostic-row-progress")]
pub(super) const FLUSH_TICK_MASK: u64 = 4095;
#[cfg(feature = "diagnostic-row-progress")]
pub(super) const FLUSH_SECONDS: u64 = 1;

#[cfg(feature = "diagnostic-row-progress")]
pub(super) const COUNTER_INPUT_ROWS: &str = "input_rows";
#[cfg(feature = "diagnostic-row-progress")]
pub(super) const COUNTER_EMITTED_ROWS: &str = "emitted_rows";
#[cfg(feature = "diagnostic-row-progress")]
pub(super) const COUNTER_PREDICATE_EVALUATIONS: &str = "predicate_evaluations";
#[cfg(feature = "diagnostic-row-progress")]
pub(super) const COUNTER_INDEX_PROBES: &str = "index_probes";
#[cfg(feature = "diagnostic-row-progress")]
pub(super) const COUNTER_INDEX_ROWS_EXAMINED: &str = "index_chain_rows_examined";
#[cfg(feature = "diagnostic-row-progress")]
pub(super) const COUNTER_EQUALITY_MATCHES: &str = "equality_matches";
#[cfg(feature = "diagnostic-row-progress")]
pub(super) const COUNTER_JOIN_CANDIDATE_PAIRS: &str = "join_candidate_pairs";
#[cfg(feature = "diagnostic-row-progress")]
pub(super) const COUNTER_KERNEL_OUTPUT_ROWS: &str = "kernel_output_rows";
#[cfg(feature = "diagnostic-row-progress")]
pub(super) const COUNTER_POSITION_ROWS_YIELDED: &str = "position_rows_yielded";

pub(super) const KEY_RULE: RecordKey = RecordKey("rule");
pub(super) const KEY_NODE: RecordKey = RecordKey("node");
pub(super) const KEY_OWNER: RecordKey = RecordKey("owner");
pub(super) const KEY_OWNER_ID: RecordKey = RecordKey("owner_id");
pub(super) const KEY_CONSUMER: RecordKey = RecordKey("consumer");
pub(super) const KEY_PLAN: RecordKey = RecordKey("plan");
pub(super) const KEY_POPULATIONS: RecordKey = RecordKey("populations");
pub(super) const KEY_ROUTE: RecordKey = RecordKey("route");
pub(super) const KEY_FAMILY: RecordKey = RecordKey("family");
pub(super) const KEY_CALLS: RecordKey = RecordKey("calls");
pub(super) const KEY_SUCCESSES: RecordKey = RecordKey("successes");
pub(super) const KEY_FAILURES: RecordKey = RecordKey("failures");
pub(super) const KEY_INDEX: RecordKey = RecordKey("index");
pub(super) const KEY_NAME: RecordKey = RecordKey("name");
#[cfg(feature = "diagnostic-row-progress")]
pub(super) const KEY_COUNTER: RecordKey = RecordKey("counter");
#[cfg(feature = "diagnostic-row-progress")]
pub(super) const KEY_COUNT: RecordKey = RecordKey("count");
#[cfg(feature = "diagnostic-row-progress")]
pub(super) const KEY_RULE_NAME: RecordKey = RecordKey("rule_name");
#[cfg(feature = "diagnostic-row-progress")]
pub(super) const KEY_INPUT_CARDINALITIES: RecordKey = RecordKey("input_cardinalities");
#[cfg(feature = "diagnostic-row-progress")]
pub(super) const KEY_NODE_NAMESPACE: RecordKey = RecordKey("node_namespace");
