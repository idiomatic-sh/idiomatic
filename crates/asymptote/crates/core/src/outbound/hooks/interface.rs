//! What the `hooks` port asks of the outside, in the core's own vocabulary.

use super::errors::HooksError;
use super::types::Installed;
use crate::domain::hooks::types::ScriptText;
use crate::domain::manifest::types::{HookName, ProductRoot};

/// What every `hooks` adapter implements.
pub trait HooksPort: std::fmt::Debug + Send + Sync {
    /// The top level of the repository the product at `root` is in: where git runs a hook from.
    ///
    /// # Errors
    ///
    /// `NotARepository` or `Git` when git cannot say where it is.
    fn top(&self, root: &ProductRoot) -> Result<ProductRoot, HooksError>;

    /// What the repository of the product at `root` holds as the hook `name`.
    ///
    /// # Errors
    ///
    /// `NotARepository` or `Git` when git cannot say where it is kept, `Unreadable` when it cannot be read.
    fn inspect(&self, root: &ProductRoot, name: &HookName) -> Result<Installed, HooksError>;

    /// Writes `script` as the hook `name`, executable, replacing what is there.
    ///
    /// # Errors
    ///
    /// `NotARepository` or `Git` when git cannot say where it is kept, `Unwritable` when it cannot be written.
    fn install(
        &self,
        root: &ProductRoot,
        name: &HookName,
        script: &ScriptText,
    ) -> Result<(), HooksError>;

    /// Removes the hook `name`; one that is not there is no failure.
    ///
    /// # Errors
    ///
    /// `NotARepository` or `Git` when git cannot say where it is kept, `Unwritable` when it cannot be removed.
    fn remove(&self, root: &ProductRoot, name: &HookName) -> Result<(), HooksError>;
}
