//! The `relation_lookup` service: free functions named by verb over the relations ryo owns, with no IO
//! and no ports. Text becomes a `Relation` here once, and a `Relation` becomes text only to leave.

use ryo_relations::domain::relations::service::relation_named;
use ryo_relations::domain::relations::types::Relation;

use super::errors::RelationLookupError;

use ryo_values::domain::descriptions::types::RelationName;

/// The relation called `text`, among the relations `known`.
///
/// # Errors
///
/// `UnknownRelation` when ryo owns no such relation.
pub(crate) fn resolve(
    known: &[Relation],
    text: &RelationName,
) -> Result<Relation, RelationLookupError> {
    relation_named(ryo_relations::domain::labels::types::NameRef(&text.0))
        .filter(|relation| known.contains(relation))
        .ok_or_else(|| RelationLookupError::UnknownRelation {
            name: text.0.to_string(),
        })
}
