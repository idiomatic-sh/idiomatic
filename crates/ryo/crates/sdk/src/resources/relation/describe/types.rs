//! The types of `relation describe`: what is asked and what comes back.

use crate::domain::language::types::LanguageName;
use crate::domain::relation::types::{ColumnName, Holds, RelationName};

/// The type of a column: `integer`, `string` or `boolean`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ColumnKind(pub Box<str>);

/// A word a column of a closed set can hold: `function`, `method`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct KindWord(pub Box<str>);

/// The name of a reason a language declares for a value it cannot resolve statically: `wildcard-import`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct UnknownName(pub Box<str>);

/// Why a language does not read a relation: the reason's name.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Reason(pub Box<str>);

/// The relation to describe.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DescribeRequest {
    /// Its name.
    pub name: RelationName,
}

/// One relation, described.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct DescribeReport {
    /// The relation asked for, and no other.
    pub relation: RelationDescription,
}

/// A relation and its columns.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct RelationDescription {
    /// The relation's name.
    pub name: RelationName,
    /// What it holds, in a sentence.
    pub summary: Holds,
    /// Its columns, in order.
    pub columns: Vec<ColumnDescription>,
    /// What each linked language answers for it, in the order they were linked.
    pub languages: Vec<LanguageDescription>,
}

/// Whether a linked language reads a relation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reading {
    /// It reads the relation.
    Reads,
    /// It does not read the relation.
    DoesNotRead,
}

/// What one linked language answers for a relation.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct LanguageDescription {
    /// The language's name.
    pub name: LanguageName,
    /// Whether it reads the relation.
    pub reads: Reading,
    /// When it does not, why: the reason's name; empty when it reads it.
    pub reason: Reason,
    /// The reasons it declares for a value it cannot resolve statically, in its own order; a column then holds `unknown("<reason>")`.
    pub unknowns: Vec<UnknownName>,
}

/// One column of a relation.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ColumnDescription {
    /// The column's name.
    pub name: ColumnName,
    /// Its type: `integer`, `string` or `boolean`.
    pub kind: ColumnKind,
    /// The words it can hold when it holds a closed set of them, in order; empty when it holds any text.
    pub words: Vec<KindWord>,
}
