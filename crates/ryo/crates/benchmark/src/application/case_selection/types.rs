//! Data owned by the partition of an audited inventory.
use ryo_benchmark_contracts::domain::workloads::types::{PreparedCase, case_count::value::CaseCount};

/// Audited global inventory and its exact selected partition.
pub struct ShardSelection {
    /// Number of cases before partitioning.
    pub global_count: CaseCount,
    /// Cases admitted to this invocation, preserving deterministic order.
    pub selected_cases: Vec<PreparedCase>,
}
