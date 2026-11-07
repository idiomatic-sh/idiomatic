//! The failure of the `contract` concept's finite callbacks.

/// Why a finite callback of the `contract` concept stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ContractError {
    /// an engine-retained error interrupted a finite semantic callback.
    #[error("an engine-retained error interrupted a finite semantic callback")]
    Interrupted,
}
