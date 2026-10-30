//! The `init` use case's failures.

use crate::outbound::hooks::errors::HooksError;
use crate::outbound::lock::errors::LockError;
use crate::outbound::manifest::errors::ManifestError;

/// Why `init` wrote nothing, or stopped.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum InitError {
    /// The product's `asymptote.toml` could not be read or written.
    #[error(transparent)]
    Manifest(#[from] ManifestError),
    /// The lock could not be written.
    #[error(transparent)]
    Lock(#[from] LockError),
    /// The repository's hooks could not be found, read or written.
    #[error(transparent)]
    Hooks(#[from] HooksError),
    /// A hook `[hooks]` lists is already a script asymptote did not write.
    #[error(
        "`{file}` is the `{hook}` hook, and asymptote did not write it: init never overwrites a hook it does not own"
    )]
    Foreign {
        /// The hook.
        hook: String,
        /// Its file.
        file: String,
    },
}
