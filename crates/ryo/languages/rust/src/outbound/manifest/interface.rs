//! What the `manifest` port asks of the outside, in the language's own vocabulary.

use super::errors::ManifestError;
use crate::domain::paths::types::FileText;
use crate::domain::targets::types::Config;
use crate::domain::targets::types::Manifest;

/// What every `manifest` adapter implements.
pub(crate) trait ManifestPort: Send + Sync {
    /// What the text of a `Cargo.toml` declares.
    ///
    /// # Errors
    ///
    /// `Unreadable` when the text is not a manifest.
    fn read(&self, text: &FileText) -> Result<Manifest, ManifestError>;

    /// What the text of a `.cargo/config.toml` says that the answer depends on.
    ///
    /// # Errors
    ///
    /// `Unreadable` when the text is not a configuration.
    fn configuration(&self, text: &FileText) -> Result<Config, ManifestError>;
}
