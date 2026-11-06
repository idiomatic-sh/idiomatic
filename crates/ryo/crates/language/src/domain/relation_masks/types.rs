//! The types of the `relation_masks` concept: a set of relations.

use super::constants::WORDS;

/// A set of relations, a bit for each, by the relation's number.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RelationSet(pub [u64; WORDS]);
