//! The types of the `query_syntax` concept: the expressions, conditions and sources a RyoQL step is written with. Plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::descriptions::types::ColumnName;
use crate::domain::query_tokens::types::{
    AliasName, ArgumentName, OperationName, QuerySpan, StepName, WordText, WrittenLiteral,
    WrittenParameter, WrittenRelation,
};
use ryo_relations::domain::relations::types::Integer;

/// What a `let` computes.
#[derive(Clone, Debug, PartialEq)]
pub enum Expression {
    /// `select ... from ... join ... where ...`.
    Select(Select),
    /// `name(argument=value, ...)`.
    Call(Call),
}

/// `select columns from source join source on a = b where filters`.
#[derive(Clone, Debug, PartialEq)]
pub struct Select {
    /// The columns kept: every one for `*`, or those listed.
    pub columns: Columns,
    /// What it reads.
    pub from: QuerySource,
    /// The join, when there is one.
    pub join: Joining,
    /// The `where` conditions, all of which must hold.
    pub filters: Vec<Condition>,
}

/// One condition of a `where`: a test, or the alternatives an `or` gives.
#[derive(Clone, Debug, PartialEq)]
pub enum Condition {
    /// One test: `name = "main"`.
    Test(Filter),
    /// `a or b and c`: at least one alternative holds, each a list of conditions that all hold. There are two or more.
    Any(Vec<Vec<Condition>>),
}

/// The columns a `select` keeps.
#[derive(Clone, Debug, PartialEq)]
pub enum Columns {
    /// `*`: every column of what it reads.
    Every,
    /// The columns the query lists.
    Listed(Vec<SelectColumn>),
}

/// Whether a `select` joins a second source to the one it reads.
#[derive(Clone, Debug, PartialEq)]
pub enum Joining {
    /// It reads one source.
    Alone,
    /// It joins this second source.
    Joined(Join),
}

/// The name a column of a `select` is kept under.
#[derive(Clone, Debug, PartialEq)]
pub enum Renaming {
    /// It keeps the name it is read under.
    Unchanged,
    /// `as` gives it this name.
    Renamed(ColumnName),
}

/// Whether the query writes an alias before a column or after a source.
#[derive(Clone, Debug, PartialEq)]
pub enum Alias {
    /// The query writes none.
    Unwritten,
    /// The query writes this one.
    Written(AliasName),
}

/// One column of a `select`, with the name it is kept under.
#[derive(Clone, Debug, PartialEq)]
pub struct SelectColumn {
    /// The column read.
    pub column: ColumnReference,
    /// The name it is kept under.
    pub rename: Renaming,
}

/// A column, with the alias of the source it belongs to when the query writes one.
#[derive(Clone, Debug, PartialEq)]
pub struct ColumnReference {
    /// `l` in `l."id"`.
    pub alias: Alias,
    /// `id` in `l."id"`.
    pub column: ColumnName,
}

/// What a `select` or a `join` reads.
#[derive(Clone, Debug, PartialEq)]
pub struct QuerySource {
    /// The relation or the earlier step.
    pub reference: SourceReference,
    /// The alias after `as`.
    pub alias: Alias,
    /// Where it is written.
    pub span: QuerySpan,
}

/// A name a query reads.
#[derive(Clone, Debug, PartialEq)]
pub enum SourceReference {
    /// A relation, written in quotes: `"syntax-tree"`.
    Relation(WrittenRelation),
    /// An earlier step, written bare: `functions`.
    Step(StepName),
}

/// `join source on l.a = r.b and l.c = r.d`.
#[derive(Clone, Debug, PartialEq)]
pub struct Join {
    /// The right side.
    pub source: QuerySource,
    /// The keys the rows are joined on, each a column of each side.
    pub on: Vec<JoinKey>,
}

/// One `l.a = r.b` of a join.
#[derive(Clone, Debug, PartialEq)]
pub struct JoinKey {
    /// The left key.
    pub left: ColumnReference,
    /// The right key.
    pub right: ColumnReference,
}

/// One `where` condition: `left test right`.
#[derive(Clone, Debug, PartialEq)]
pub struct Filter {
    /// The left operand.
    pub left: WrittenOperand,
    /// How they are compared.
    pub test: WrittenTest,
    /// The right operand.
    pub right: WrittenOperand,
    /// Where it is written.
    pub span: QuerySpan,
}

/// How a filter compares its operands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WrittenTest {
    /// `=`.
    Equal,
    /// `!=`.
    NotEqual,
    /// `<`.
    LessThan,
    /// `<=`.
    LessThanOrEqual,
    /// `>`.
    GreaterThan,
    /// `>=`.
    GreaterThanOrEqual,
    /// `in`.
    In,
    /// `not in`.
    NotIn,
    /// `glob_any(left, right)`.
    GlobAny,
    /// `not glob_any(left, right)`.
    NotGlobAny,
    /// `float_text_gte(left, right)`: numbers written as text, compared as numbers.
    FloatTextGreaterThanOrEqual,
    /// `starts_with(text, prefix)`.
    StartsWith,
    /// `ends_with(text, suffix)`.
    EndsWith,
    /// `matches(text, pattern)`: a regular expression searches the text.
    Matches,
    /// `numeric_gte(left, right)`: exact integer and binary64 numeric ordering.
    NumericGreaterThanOrEqual,
    /// `integer_successor(left, right)`: right is exactly left plus one; integers only.
    IntegerSuccessor,
    /// `natural_lt(left, right)`: case-sensitive natural text order; ASCII digit runs,
    /// fraction-style zero prefixes, Unicode whitespace separators, no normalization.
    NaturalLessThan,
}

/// One side of a filter.
#[derive(Clone, Debug, PartialEq)]
pub enum WrittenOperand {
    /// A column.
    Column(ColumnReference),
    /// `$name`.
    Parameter(WrittenParameter),
    /// `"text"`.
    Text(WrittenLiteral),
    /// `42`.
    Number(Integer),
    /// `pascal(pkg)`: a function applied to operands.
    Function(WrittenApplied),
}

/// A function a filter applies to text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WrittenFunction {
    /// `concat(a, b, ...)`.
    Concat,
    /// `pascal(text)`.
    Pascal,
    /// `camel(text)`.
    Camel,
    /// `snake(text)`.
    Snake,
    /// `kebab(text)`.
    Kebab,
    /// `lower(text)`: Unicode lowercase text without changing separators.
    Lower,
}

/// `concat(pascal(pkg), "Service")`: a function and the operands it is applied to.
#[derive(Clone, Debug, PartialEq)]
pub struct WrittenApplied {
    /// The function.
    pub function: WrittenFunction,
    /// Its arguments, in the order written.
    pub arguments: Vec<WrittenOperand>,
    /// Where it is written.
    pub span: QuerySpan,
}

/// `name(argument=value, ...)`.
#[derive(Clone, Debug, PartialEq)]
pub struct Call {
    /// The operation's name.
    pub name: OperationName,
    /// Its arguments, in the order written.
    pub arguments: Vec<Argument>,
    /// Where it is written.
    pub span: QuerySpan,
}

/// `name=value`.
#[derive(Clone, Debug, PartialEq)]
pub struct Argument {
    /// The argument's name.
    pub name: ArgumentName,
    /// Its value.
    pub value: ArgumentValue,
}

/// The value of an argument.
#[derive(Clone, Debug, PartialEq)]
pub enum ArgumentValue {
    /// `"text"`.
    Text(WrittenLiteral),
    /// `42`.
    Number(Integer),
    /// A bare word: a column, a step or a keyword such as `descendant`.
    Word(WordText),
    /// A relation input resolved to one exact step definition by the linker.
    Step(StepName),
    /// `$name`.
    Parameter(WrittenParameter),
    /// `[a, b]`.
    List(Vec<ArgumentValue>),
    /// `{key: value}`.
    Map(Vec<Argument>),
}
