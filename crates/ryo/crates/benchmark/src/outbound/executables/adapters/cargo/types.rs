//! What the cargo adapter decides with.

/// The platform the operation runs on, as it bears on a nested build.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BuildPlatform {
    /// A Unix: the outer build replaces itself with the collector, so no build holds the lock.
    Unix,
    /// Another platform: the outer build waits on the collector and keeps the lock a nested build needs.
    Other,
}
