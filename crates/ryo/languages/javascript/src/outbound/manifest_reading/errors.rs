//! A manifest the project parser cannot read.

/// Why a manifest reading adapter could not read a manifest.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum ManifestReadingError {
    /// The manifest's text is not the format the adapter reads; the reader's message says where.
    #[error("{0}")]
    Unreadable(Box<str>),
}
