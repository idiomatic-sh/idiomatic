//! How a measured run treats the environment its linter pins.

/// Whether a run keeps the environment a linter pins for comparable timing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pinning {
    /// A timing measurement: the linter's pins (one worker thread) stand, so contestants compare.
    Timed,
    /// A correctness comparison: the child runs in the caller's environment, at default threads.
    Untimed,
}
