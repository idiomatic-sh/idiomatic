//! The types of the `descriptions` concept: what ryo says of the relations and the operators it owns. Plain data, public fields and derives, with no constructor and no inherent impl.

use ryo_relations::domain::relations::types::Holds;
use ryo_relations::domain::terms::types::UnknownName;

/// Why a language does not read a relation, by the reason's name; empty when it reads it.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AbsenceReason(pub Box<str>);

/// A column's type: `integer`, `string` or `boolean`.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ColumnKind(pub Box<str>);

/// Whether a language reads a relation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelationReadability {
    /// It reads the relation.
    Reads,
    /// It does not.
    DoesNotRead,
}

/// A relation and its columns.
#[derive(Clone, Debug, PartialEq, Eq)]
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

/// What one linked language answers for a relation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LanguageDescription {
    /// The language's name.
    pub name: LanguageName,
    /// Whether it reads the relation.
    pub reads: RelationReadability,
    /// When it does not, why: the reason's name; empty when it reads it.
    pub reason: AbsenceReason,
    /// The reasons it declares for a value it cannot resolve statically, in its own order.
    pub unknowns: Vec<UnknownName>,
}

/// One column of a relation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColumnDescription {
    /// The column's name.
    pub name: ColumnName,
    /// Its type: `integer`, `string` or `boolean`.
    pub kind: ColumnKind,
    /// The words it can hold when it holds a closed set of them, in order; empty when it holds any text.
    pub words: Vec<KindWord>,
}

/// One line about a relation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelationSummary {
    /// The relation's name.
    pub name: RelationName,
    /// What it holds, in a sentence.
    pub summary: Holds,
}

/// An operator's name, as a query writes it: `join`, `group-count`.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OperatorName(pub Box<str>);

/// What an operator does, in a sentence.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Purpose(pub Box<str>);

/// One closed choice an operator offers, on one line.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OptionLine(pub Box<str>);

/// An operator written as one statement of query text.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Example(pub Box<str>);

/// A field's name.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FieldName(pub Box<str>);

/// Whether an operator cannot be written without a field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldRequirement {
    /// The operator cannot be written without it.
    Required,
    /// The operator can be written without it.
    Optional,
}

/// What an operator is, what it takes and an example.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperatorDescription {
    /// The name a query writes.
    pub name: OperatorName,
    /// What it does, in a sentence.
    pub summary: Purpose,
    /// The fields it takes.
    pub fields: Vec<FieldDescription>,
    /// The closed choices it offers, one line each, for an operator that has any.
    pub options: Vec<OptionLine>,
    /// The operator written as one statement of query text.
    pub example: Example,
}

/// One field of an operator.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldDescription {
    /// The field's name.
    pub name: FieldName,
    /// What it is, in a phrase.
    pub summary: Purpose,
    /// Whether the operator cannot be written without it.
    pub required: FieldRequirement,
}

/// A language's name.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LanguageName(pub Box<str>);

/// The name of a relation: one ryo owns, or one a host declares.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RelationName(pub Box<str>);

/// A word a rule writes for a kind of a node or a token, or the name of a kind a language has: a name, or a pattern.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KindWord(pub Box<str>);

/// The kinds one linked language has: the words a query can write for a kind of its nodes and tokens.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KindVocabulary {
    /// The language.
    pub language: LanguageName,
    /// The names of its kinds, each once.
    pub kinds: Vec<KindWord>,
}

/// The kinds of every linked language, in the order they were linked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KindVocabularies(pub Vec<KindVocabulary>);

/// The name of a production of the RyoQL grammar: the parser that reads it.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProductionName(pub Box<str>);

/// How a production is written, on one line: its words, marks and the productions it is made of.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProductionForm(pub Box<str>);

/// One production of the RyoQL grammar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Production {
    /// The parser that reads it.
    pub name: ProductionName,
    /// How it is written.
    pub form: ProductionForm,
    /// Text it reads, whole.
    pub example: Example,
}

/// The RyoQL grammar: every production, from the whole program down to a word.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Grammar(pub Vec<Production>);

/// How many steps a query compiles to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StepCount(pub usize);

/// What a valid query uses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidateReport {
    /// The relations it reads, each once.
    pub relations: Vec<RelationName>,
    /// The operations it uses, each once.
    pub operators: Vec<OperatorName>,
    /// How many steps it compiles to.
    pub steps: StepCount,
}

/// The name of a column, as the query writes it or keeps it.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ColumnName(pub Box<str>);
