//! The `visit` concept's failures.

use ryo_language::domain::relations::types::Relation;

/// Why a visit could not be planned.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum VisitError {
    /// A query reads a relation the catalog does not list.
    #[error("a query reads a relation the catalog does not list")]
    UnknownRelation {
        /// The relation.
        relation: Relation,
    },
}
