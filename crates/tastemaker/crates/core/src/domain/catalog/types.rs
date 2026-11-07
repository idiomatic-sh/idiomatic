//! The types of the `catalog` concept: what the engine says of the relations and operators a query reads and calls, and of
//! the grammar it is written in. Plain data, public fields and derives, with no constructor and no inherent impl.

/// A name in the engine's catalog: a relation, a column, an operator, a field, a word, a language.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct CatalogName(pub Box<str>);

/// What the catalog says of something, in a phrase or a line of query text.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct CatalogText(pub Box<str>);

/// One relation or operator, on one line.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CatalogEntry {
    /// Its name.
    pub name: CatalogName,
    /// What it is.
    pub summary: CatalogText,
}

/// A relation: its columns and what each linked language answers for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelationPage {
    /// The relation.
    pub entry: CatalogEntry,
    /// Its columns, in order.
    pub columns: Vec<CatalogColumn>,
    /// Each linked language, in the order linked.
    pub languages: Vec<CatalogLanguage>,
}

/// One column of a relation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatalogColumn {
    /// Its name.
    pub name: CatalogName,
    /// Its type.
    pub kind: CatalogName,
    /// The words it can hold, for a column of a closed set; empty otherwise.
    pub words: Vec<CatalogName>,
}

/// What one linked language answers for a relation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatalogLanguage {
    /// The language.
    pub name: CatalogName,
    /// Whether it reads the relation.
    pub reading: Reading,
    /// The reasons it writes an unknown value for, when it cannot resolve one.
    pub unknowns: Vec<CatalogName>,
}

/// Whether a language reads a relation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reading {
    /// It reads it.
    Reads,
    /// It does not, for this reason.
    Absent(CatalogName),
}

/// An operator: its fields, its choices and an example.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperatorPage {
    /// The operator.
    pub entry: CatalogEntry,
    /// Its fields.
    pub fields: Vec<CatalogField>,
    /// The closed choices it offers, a line each.
    pub options: Vec<CatalogText>,
    /// It written as one statement.
    pub example: CatalogText,
}

/// One field of an operator.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatalogField {
    /// Its name.
    pub name: CatalogName,
    /// Whether the operator cannot be written without it.
    pub requirement: Requirement,
    /// What it is.
    pub summary: CatalogText,
}

/// Whether an operator cannot be written without a field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Requirement {
    /// It cannot.
    Required,
    /// It can.
    Optional,
}

/// One production of the query grammar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Production {
    /// Its name.
    pub name: CatalogName,
    /// Its form.
    pub form: CatalogText,
    /// An example the parser reads.
    pub example: CatalogText,
}

/// Relations or operators, a line each.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CatalogEntries(pub Vec<CatalogEntry>);

/// What the catalog answers for a relation's name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RelationAnswer {
    /// The relation of that name.
    Found(RelationPage),
    /// No relation has the name.
    Unknown,
}

/// What the catalog answers for an operator's name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OperatorAnswer {
    /// The operator of that name.
    Found(OperatorPage),
    /// No operator has the name.
    Unknown,
}

/// The grammar a query is written in, a production each.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Grammar(pub Vec<Production>);
