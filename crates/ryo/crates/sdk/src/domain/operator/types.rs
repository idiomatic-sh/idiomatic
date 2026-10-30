//! The types of the `operator` concept: an operation a query uses by its name, what it does and how it is written.

/// The name of an operation a query uses.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OperatorName(pub Box<str>);

/// What an operator, or one of its fields, does, in a sentence or a phrase.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Purpose(pub Box<str>);

/// An operator written as one statement of query text.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Example(pub Box<str>);
