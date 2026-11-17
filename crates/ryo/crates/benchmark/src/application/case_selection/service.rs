//! Select a shard only after the complete inventory is loaded.
use super::types::ShardSelection;
use ryo_benchmark_contracts::domain::requests::types::CaseSelection;
use ryo_benchmark_contracts::domain::workloads::types::{
    PreparedCase, case_count::value::CaseCount,
};

/// Partition the sorted inventory into this shard.
#[must_use]
pub fn select(mut cases: Vec<PreparedCase>, selection: CaseSelection) -> ShardSelection {
    cases.sort_by(|left, right| (&left.rule, &left.name).cmp(&(&right.rule, &right.name)));
    let global_count = cases.len();
    let assigned = match selection.shard {
        ryo_benchmark_contracts::domain::requests::types::ShardPartition::Shard(shard) => {
            shard.select(cases).collect::<Vec<_>>()
        }
        ryo_benchmark_contracts::domain::requests::types::ShardPartition::All => cases,
    };
    ShardSelection {
        global_count: CaseCount(global_count),
        selected_cases: assigned,
    }
}
