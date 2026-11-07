//! The choices one verification run is given.
use crate::outbound::receipts::types::CaseReceipts;
use ryo_benchmark_contracts::domain::requests::types::{CaseId, ShardPartition};
use std::collections::BTreeSet;

/// The partition, case subset and receipt destination a verification run covers and records.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaseRun {
    /// Where the raw case receipts are appended, if anywhere.
    pub receipts: CaseReceipts,
    /// The shard of the case inventory this run covers.
    pub shard: ShardPartition,
    /// The durable case IDs of that shard this run covers.
    pub ids: SelectedIds,
}

/// Which durable case IDs of the chosen shard a run covers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SelectedIds {
    /// The whole shard.
    WholeShard,
    /// Only these case IDs.
    Subset(BTreeSet<CaseId>),
}

/// One flag of a verification run's command line, as the user wrote it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaseArgument(pub String);
