//! The choices one verification run is given.
use crate::outbound::receipts::types::CaseReceipts;
use ryo_benchmark_contracts::domain::requests::types::ShardPartition;

/// The partition and receipt destination a verification run covers and records.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaseRun {
    /// Where the raw case receipts are appended, if anywhere.
    pub receipts: CaseReceipts,
    /// The shard of the case inventory this run covers.
    pub shard: ShardPartition,
}

/// One flag of a verification run's command line, as the user wrote it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaseArgument(pub String);
