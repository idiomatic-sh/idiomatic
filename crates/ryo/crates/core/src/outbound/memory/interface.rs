//! What the `memory` port asks of the outside: how much memory the machine has to spare.

use super::errors::MemoryError;
use ryo_values::domain::runs::types::MemoryBytes;

/// What every `memory` adapter implements.
pub trait MemoryPort: Send + Sync {
    /// The most memory the run may use: what the edge asked for, at most what the machine has to spare, and what it has when no
    /// limit was asked.
    ///
    /// # Errors
    ///
    /// `Unreadable` when the machine cannot say, `NoneAvailable` when it has none to spare, `MoreThanAvailable` when more was asked
    /// than it has.
    fn budget(&self) -> Result<MemoryBytes, MemoryError>;

    /// The bytes of memory this process holds resident now.
    ///
    /// # Errors
    ///
    /// `Unreadable` when the machine cannot say.
    fn resident_bytes(&self) -> Result<MemoryBytes, MemoryError>;
}
