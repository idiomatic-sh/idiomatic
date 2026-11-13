//! The `facts` port's failures.

/// Why the `facts` port failed.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum FactsError {
    /// Where the rows or the words are kept refused or failed.
    #[error("the facts store failed: {reason}")]
    Store {
        /// What it said.
        reason: String,
    },
    /// A file the words are kept in failed.
    #[error("a file of the facts failed: {reason}")]
    File {
        /// What the system said.
        reason: String,
    },
    /// A source of the file's rows refused them; the reason stays with the source, which reports it as it is.
    #[error("a source of the file's rows refused them")]
    Refused,
    /// Two sources of one file serve the same relation, so its rows would have two producers.
    #[error("two sources of the file serve the relation {relation:?}")]
    Overlap {
        /// The first relation, in catalog order, that both serve.
        relation: ryo_relations::domain::relations::types::Relation,
    },
}
