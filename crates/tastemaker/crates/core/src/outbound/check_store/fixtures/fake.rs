//! The fake of the `check_store` port: the answer kept in memory.

use std::sync::Mutex;

use crate::outbound::check_store::errors::CheckStoreError;
use crate::outbound::check_store::interface::CheckStorePort;
use crate::outbound::check_store::types::Checked;

/// What the core's tests stand in for `check_store` with.
#[derive(Debug, Default)]
pub struct CheckStoreFake {
    /// The answer kept.
    pub checks: Mutex<Checked>,
    /// How many times it was kept.
    pub check_saves: Mutex<usize>,
    /// Whether keeping fails, as a store that cannot be written does.
    pub unwritable: Mutex<bool>,
}

impl CheckStorePort for CheckStoreFake {
    fn checked(&self) -> Checked {
        self.checks
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    fn keep_checked(&self, checked: &Checked) -> Result<(), CheckStoreError> {
        *self
            .check_saves
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) += 1;
        if *self
            .unwritable
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
        {
            return Err(CheckStoreError::Unwritable {
                file: "/fake/.tastemaker_cache".to_owned(),
                reason: "the fake refuses every save".to_owned(),
            });
        }
        checked.clone_into(
            &mut self
                .checks
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        );
        Ok(())
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
