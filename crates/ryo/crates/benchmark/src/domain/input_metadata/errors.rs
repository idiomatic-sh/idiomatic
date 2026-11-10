//! Why declared input metadata is refused.

/// A refused input metadata declaration.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum InputMetadataError {
    /// Refuses: metadata names a source the case never declared.
    #[error("metadata names an undeclared source")]
    UndeclaredSource,
    /// Refuses: a required observation is missing or does not cover every declared source.
    #[error("required file observations are missing")]
    RequiredObservationsMissing,
}
