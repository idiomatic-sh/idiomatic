//! How a measured run treats the environment its linter pins.

/// Whether a run keeps the environment a linter pins for comparable timing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Pinning {
    /// A timing measurement: the linter's pins (one worker thread) stand, so contestants compare.
    Timed,
    /// A correctness comparison: the thread pin is dropped, so the child runs at its default threads.
    Untimed,
}
