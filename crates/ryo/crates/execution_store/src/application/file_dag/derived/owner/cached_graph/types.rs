//! One request-bound graph plan with invocation-local traversal buffers. Exact cache identity
//! and bounded retention stay in `CacheBackend` and the graph tables of the owner.
use crate::application::file_dag::derived::owner::graphs::types::handles::service::{
    DomainHandle, GraphHandle,
};
use ryo_execution_domain::domain::stored_graph::types::GraphWalk;
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Direction};

/// How many leading request columns partition a reach demand.
#[derive(Clone, Copy, Debug)]
pub(super) struct PartitionWidth(pub(super) usize);

/// A position within the left key, by which one request column is bound.
#[derive(Clone, Copy, Debug)]
pub(super) struct KeyPosition(pub(super) usize);
