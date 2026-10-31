//! Builds the execution adapters from dependencies chosen by the composition.
use super::adapters::process::adapter::ExecutionProcessAdapter;
/// Construct the process implementation of the execution contract.
#[must_use]
pub fn process() -> impl super::interface::ExecutionPort {
    ExecutionProcessAdapter
}

/// Construct isolated per-child Linux CPU and peak-RSS instrumentation.
/// The helper launches the actual child from a small resident process. Linux includes
/// that small pre-exec resident footprint as a floor; it excludes the parent harness.
/// Wall and CPU measure the actual child, excluding helper startup and protocol IO.
#[must_use]
#[cfg(not(test))]
pub fn timed(
    executable: std::path::PathBuf,
    digest: crate::domain::suite::types::ArtifactDigest,
) -> impl super::interface::ExecutionPort {
    super::adapters::timed::adapter::ExecutionTimedAdapter::new(
        super::adapters::timed::settings::ExecutionTimedSettings { executable, digest },
    )
}

/// Direct resource observation, exclusively for the fresh collector process.
/// A resident caller's memory can otherwise contaminate Linux exec-time RSS.
#[must_use]
pub fn direct() -> impl super::interface::ExecutionPort {
    super::adapters::direct::adapter::ExecutionDirectAdapter
}
