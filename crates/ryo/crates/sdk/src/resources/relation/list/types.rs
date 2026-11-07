//! The types of `relation list`: what is asked and what comes back.

use crate::domain::relation::types::{Holds, RelationName};

/// Every relation ryo owns, in name order. It names no language: which languages are linked is not ryo's to say.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ListReport {
    /// The relations.
    pub relations: Vec<RelationSummary>,
}

/// One line about a relation.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct RelationSummary {
    /// The relation's name.
    pub name: RelationName,
    /// What it holds, in a sentence.
    pub summary: Holds,
}
