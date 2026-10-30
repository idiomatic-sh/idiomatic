//! The `memory` port's failures.

/// Why the `memory` port failed.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum MemoryError {
    /// The machine's memory could not be read.
    #[error("cannot read the machine's memory: {reason}")]
    Unreadable {
        /// What went wrong.
        reason: String,
    },
    /// The system reports no available memory.
    #[error("the system reports no available memory")]
    NoneAvailable,
    /// More memory was asked for than the machine has to spare.
    #[error("{asked} bytes asked for, {available} available")]
    MoreThanAvailable {
        /// The bytes asked for.
        asked: u64,
        /// The bytes available.
        available: u64,
    },
}
