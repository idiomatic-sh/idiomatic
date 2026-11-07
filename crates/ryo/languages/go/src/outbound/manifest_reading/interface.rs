//! What the `manifest_reading` port asks of the outside: the text of a manifest in, what it declares out.

use super::errors::ManifestReadingError;
use crate::domain::modules::types::{Declared, Manifestness};
use ryo::languages::ManifestText;

/// What every `project` adapter implements.
pub(crate) trait ManifestReadingPort: Send + Sync {
    /// What the manifest of the kind `kind` with the text `text` declares: the module it names, the modules it requires and replaces,
    /// and the directories a workspace uses.
    ///
    /// # Errors
    ///
    /// `NoModule`, `Unopened`, `Unclosed`, `Incomplete` or `Unquoted` when the text is not a manifest the tool reads.
    fn declared(
        &self,
        kind: Manifestness,
        text: &ManifestText,
    ) -> Result<Declared, ManifestReadingError>;
}
