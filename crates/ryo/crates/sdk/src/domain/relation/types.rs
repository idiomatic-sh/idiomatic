//! The types of the `relation` concept: a relation by its name, a column of one, and what one holds.

/// The name of a relation a query reads: one ryo owns, or one the host declares.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RelationName(pub Box<str>);

/// The name of a column: of a relation, or the heading of a column of a finding's row.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ColumnName(pub Box<str>);

/// What a relation holds, in a sentence or a phrase.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Holds(pub Box<str>);
