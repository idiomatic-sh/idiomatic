//! The failure of the `equality_join` concept's finite callbacks.

/// Why a finite callback of the `equality_join` concept stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum EqualityJoinError {
    /// the engine interrupted an equality join; its own error is retained outside the capability.
    #[error("the engine interrupted an equality join; its own error is retained outside the capability")]
    Interrupted,
}
