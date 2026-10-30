//! The `list_relations` use case's failures.

use crate::outbound::language::errors::LanguageError;

/// Why `list_relations` failed.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ListRelationsError {
    /// The relation catalog could not be read.
    #[error("the relation catalog failed: {0}")]
    Language(#[from] LanguageError),
}
