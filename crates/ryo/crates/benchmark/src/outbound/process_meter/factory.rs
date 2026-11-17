//! Builds the process meter adapter by constructor injection; called only by an edge.

/// The meter that reads this process's clock and `/proc/self` on Linux.
#[must_use]
pub fn linux() -> impl super::interface::ProcessMeterPort {
    super::adapters::linux::adapter::ProcessMeterLinuxAdapter
}
