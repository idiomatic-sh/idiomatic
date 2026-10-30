//! The fake of the `memory` port: what the core's tests stand in for it with.

use super::super::errors::MemoryError;
use super::super::interface::MemoryPort;
use ryo_values::domain::runs::types::{Budget, MemoryBytes};

/// What the core's tests stand in for `memory` with: it has the bytes it was given, lets a run use what was asked of them, says none
/// is available when it was given none, or fails.
#[derive(Clone, Copy, Debug)]
pub struct MemoryFake {
    /// The bytes the machine has to spare.
    pub available: u64,
    /// The most memory the caller asked a run may use.
    pub asked: Budget,
    /// The bytes it reports the process holds.
    pub resident: u64,
    /// Whether it fails instead.
    pub fails: bool,
}

impl Default for MemoryFake {
    /// A machine with a gigabyte to spare.
    fn default() -> Self {
        Self {
            available: 1 << 30,
            asked: Budget::Machine,
            resident: 1 << 20,
            fails: false,
        }
    }
}

impl MemoryPort for MemoryFake {
    fn budget(&self) -> Result<MemoryBytes, MemoryError> {
        if self.fails {
            return Err(MemoryError::Unreadable {
                reason: "the fake was told to fail".to_owned(),
            });
        }
        if self.available == 0 {
            return Err(MemoryError::NoneAvailable);
        }
        match self.asked {
            Budget::Machine => Ok(MemoryBytes(self.available)),
            Budget::Bytes(asked) if asked.0 > self.available => {
                Err(MemoryError::MoreThanAvailable {
                    asked: asked.0,
                    available: self.available,
                })
            }
            Budget::Bytes(asked) => Ok(asked),
        }
    }

    fn resident_bytes(&self) -> Result<MemoryBytes, MemoryError> {
        if self.fails {
            return Err(MemoryError::Unreadable {
                reason: "the fake was told to fail".to_owned(),
            });
        }
        Ok(MemoryBytes(self.resident))
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
