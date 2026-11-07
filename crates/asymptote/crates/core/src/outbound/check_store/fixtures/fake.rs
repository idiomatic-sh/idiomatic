//! The fake of the `check_store` port: what the store keeps, in memory.

use std::sync::Mutex;

use crate::outbound::check_store::errors::CheckStoreError;
use crate::outbound::check_store::interface::CheckStorePort;
use crate::outbound::check_store::types::Stored;

/// What the core's tests stand in for `check_store` with.
#[derive(Debug, Default)]
pub struct CheckStoreFake {
    /// What is kept.
    pub stored: Mutex<Stored>,
    /// The number of writes, including ones that repeat what is kept.
    pub saves: std::sync::atomic::AtomicUsize,
    /// Whether every write is refused, as a store directory that cannot be written refuses it.
    pub refusing: bool,
}

impl CheckStoreFake {
    /// What is kept, read.
    #[must_use]
    pub fn kept(&self) -> Stored {
        self.stored
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

impl CheckStorePort for CheckStoreFake {
    fn load(&self) -> Stored {
        self.kept()
    }

    fn keep(&self, stored: &Stored) -> Result<(), CheckStoreError> {
        if self.refusing {
            return Err(CheckStoreError::Unwritable {
                directory: "/fake".to_owned(),
                reason: "the fake refuses every write".to_owned(),
            });
        }
        let _saves_before = self
            .saves
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        *self
            .stored
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = stored.clone();
        Ok(())
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
