//! The `manifest_reading` port's failures.

/// Why an operation of `project` failed.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum ManifestReadingError {
    /// The tool that reads manifests could not read one.
    #[error("{reason}")]
    Unreadable {
        /// What the tool said.
        reason: String,
    },
}
