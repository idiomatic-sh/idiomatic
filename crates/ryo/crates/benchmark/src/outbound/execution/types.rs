//! Plain data of the execution port.

/// How a port implementation observes the numeric exit of a completed child.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExitObservationMode {
    /// Numeric exits are directly observed with no synthetic timeout status.
    Direct,
    /// Wrapped, bounded and unknown observers cannot certify process failures.
    Mediated,
}

/// The exit of a completed child, decoded from the operating system status.
pub(crate) mod process_exit;

/// Where a worker's request is stored, which the in-process worker reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredRequest(pub std::path::PathBuf);

/// The compile time of each rule a stored request names.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RuleCompileTimes(pub std::collections::BTreeMap<ryo::linter::RuleId, std::time::Duration>);
