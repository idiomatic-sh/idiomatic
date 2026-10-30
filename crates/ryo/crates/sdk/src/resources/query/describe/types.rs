//! The types of `query describe`: what comes back.

use crate::domain::operator::types::Example;

/// The name of a production of the grammar: `select`, `filter`, `word`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ProductionName(pub Box<str>);

/// How a production is written, on one line: quoted text is written as is, a bare lowercase word is a keyword or another
/// production.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Form(pub Box<str>);

/// The grammar of RyoQL.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct DescribeReport {
    /// Every production, from the whole program down to a word.
    pub productions: Vec<Production>,
}

/// One production of the grammar.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Production {
    /// Its name.
    pub name: ProductionName,
    /// How it is written.
    pub form: Form,
    /// Text it reads, whole.
    pub example: Example,
}
