//! The `relation_lookup` concept's failures.

/// Why a question of the catalog failed.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum RelationLookupError {
    /// No relation has that name.
    #[error("no relation named `{name}`")]
    UnknownRelation {
        /// The name asked for: text that entered the engine.
        name: String,
    },
}
