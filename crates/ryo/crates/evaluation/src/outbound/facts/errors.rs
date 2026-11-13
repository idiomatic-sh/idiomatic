//! The errors of the `facts` port.

use ryo_relations::domain::relations::types::Relation;

/// What a `facts` adapter refuses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum FactsError {
    /// No adapter of the composition makes the relation.
    #[error("no facts adapter makes {relation:?}")]
    Unserved {
        /// The relation asked for.
        relation: Relation,
    },
}
