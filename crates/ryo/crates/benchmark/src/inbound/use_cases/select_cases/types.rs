//! The choices one verification run is given.
use crate::outbound::receipts::types::CaseReceipts;
use benchmark_contracts::domain::requests::types::ShardPartition;

/// The partition and receipt destination a verification run covers and records.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaseRun {
    /// Where the raw case receipts are appended, if anywhere.
    pub receipts: CaseReceipts,
    /// The shard of the case inventory this run covers.
    pub shard: ShardPartition,
    /// Where the time each rule's cases took is written as BMF, if anywhere.
    pub timings: VerifyTimes,
}

/// Where a run writes how long its rules took to verify.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VerifyTimes {
    /// The run writes no times.
    Unrecorded,
    /// The run writes its times, as BMF, to this file.
    Recorded(std::path::PathBuf),
}

/// One flag of a verification run's command line, as the user wrote it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaseArgument(pub String);
