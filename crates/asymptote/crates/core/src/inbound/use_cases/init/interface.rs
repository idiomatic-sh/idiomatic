//! What the `init` use case promises its callers.

use super::errors::InitError;
use super::types::{InitReport, InitRequest};

/// What `init` offers its callers.
pub trait InitUseCase: Send + Sync {
    /// Writes a minimal `asymptote.toml` and an empty lock where the product has none, and installs one script in the
    /// repository for each hook `[hooks]` lists, rewriting its own, removing its own for hooks no longer listed, and
    /// never touching another's.
    ///
    /// # Errors
    ///
    /// `Foreign` when a listed hook is a script asymptote did not write, before anything is written; `Hooks` when
    /// the directory is in no git repository or a script cannot be read or written; `Manifest` or `Lock` when the
    /// configuration cannot be read or written.
    fn init(&self, request: InitRequest) -> Result<InitReport, InitError>;
}
