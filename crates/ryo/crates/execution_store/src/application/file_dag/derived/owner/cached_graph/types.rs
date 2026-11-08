//! One request-bound graph plan with invocation-local traversal buffers. Exact cache identity
//! and bounded retention stay in `CacheBackend` and the graph tables of the owner.
use crate::application::file_dag::derived::owner::graphs::types::handles::service::{
    DomainHandle, GraphHandle,
};
use ryo_execution_domain::domain::stored_graph::types::GraphWalk;
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Direction};

/// The graph an axis or reach operator reads, cached for its invocation.
pub enum CachedGraph {
    Reach(ReachGraph),
    Axis(AxisGraph),
}

/// How many leading request columns partition a reach demand.
#[derive(Clone, Copy, Debug)]
pub(super) struct PartitionWidth(pub(super) usize);

/// A position within the left key, by which one request column is bound.
#[derive(Clone, Copy, Debug)]
pub(super) struct KeyPosition(pub(super) usize);

pub(crate) struct ReachGraph {
    pub(super) access: GraphHandle,
    pub(super) left_on: Vec<ColumnIndex>,
    pub(super) bindings: Vec<KeyPosition>,
    pub(super) partition_width: PartitionWidth,
    pub(super) key: Vec<Value>,
    pub(super) request: Vec<Value>,
    pub(super) destinations: Vec<Value>,
    pub(super) walk: GraphWalk,
}

pub(crate) struct AxisGraph {
    pub(super) access: GraphHandle,
    pub(super) domain: DomainHandle,
    pub(super) id: ColumnIndex,
    pub(super) direction: Direction,
    pub(super) destinations: Vec<Value>,
    pub(super) walk: GraphWalk,
}
