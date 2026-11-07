//! The types of the `list_relations` use case: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo_values::domain::descriptions::types::RelationSummary;

/// Every relation ryo owns, in name order. It names no language: which languages are linked is not ryo's to say.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListRelationsReport {
    /// The relations.
    pub relations: Vec<RelationSummary>,
}
