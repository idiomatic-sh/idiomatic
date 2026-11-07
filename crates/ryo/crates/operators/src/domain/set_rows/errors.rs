//! The failure of the `set_rows` concept's finite callbacks.

/// Why a finite callback of the `set_rows` concept stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum SetError {
    /// the engine interrupted a set operation; its own error is retained with the driver.
    #[error("the engine interrupted a set operation; its own error is retained with the driver")]
    Interrupted,
}
