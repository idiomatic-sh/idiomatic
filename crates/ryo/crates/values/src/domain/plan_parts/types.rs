//! The types of the `plan_parts` concept: the numbers, text and comparisons a query plan is made of. Plain data, public fields and derives, with no constructor and no inherent impl.

use ryo_relations::domain::relations::types::{Integer, Relation, Span};

/// The text of a RyoQL query.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QueryText(pub Box<str>);

/// Text the query wrote: a string, a path or a glob.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LiteralText(pub Box<str>);

/// A parameter's name, as the host supplies it.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ParameterName(pub Box<str>);

/// A key of the tables a `list<table>` parameter holds, naming the column its values fill: text from the query.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TableKey(pub Box<str>);

/// A pattern with `{slot}` where text is captured.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CapturePattern(pub Box<str>);

/// Whether a projection drops the rows that repeat.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RepeatedRows {
    /// A row that repeats is kept once.
    Dropped,
    /// Every row is kept: no row repeats, because a column kept says which row it is.
    Kept,
}

/// An input of a plan's frontier, by its place among them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InputNumber(pub usize);

/// A step of the plan, by its place in the plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StepId(pub u32);

/// A column, by its place in the table that holds it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ColumnIndex(pub u32);

/// A parameter, by its place among the plan's parameters.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ParameterId(pub u32);

/// A whole number a cell or a parameter holds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WholeNumber(pub i64);

/// A yes or no a cell or a parameter holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FlagState {
    /// Yes.
    Raised,
    /// No.
    Lowered,
}

/// What a step reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Input {
    /// A relation ryo owns.
    Relation(Relation),
    /// An earlier step.
    Step(StepId),
    /// An explicit typed producer binding, resolved by the request graph rather than the relation catalog.
    BoundObservation(crate::domain::plan_parts::types::ObservationBindingId),
}

/// Which side of a join a column comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    /// The left input.
    Left,
    /// The right input.
    Right,
}

/// Which way an axis walks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Direction {
    /// To the parent.
    Parent,
    /// To the children.
    Child,
    /// To every ancestor.
    Ancestor,
    /// To every descendant.
    Descendant,
    /// To the nearest ancestor in the domain.
    NearestAncestor,
    /// To every descendant below the children of the starting entity: the children of the descendants.
    Deep,
    /// To the sibling just before the starting entity under its parent: the syntax tree's own order.
    PreviousSibling,
    /// To the sibling just after the starting entity under its parent: the syntax tree's own order.
    NextSibling,
}

/// A column a join keeps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JoinColumn {
    /// The input it comes from.
    pub side: Side,
    /// Its place in that input.
    pub column: ColumnIndex,
}

/// Physical source-keyed lookup of a positive transitive closure, internal to a compiled join.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReachLookup {
    /// Physical edge column holding the closure source.
    pub source: ColumnIndex,
    /// Physical edge column holding the closure destination.
    pub destination: ColumnIndex,
    /// Physical partition columns, in the original closure's argument order.
    pub keys: Vec<ColumnIndex>,
    /// Logical RHS output map over partition keys followed by source and destination.
    pub columns: Vec<ColumnIndex>,
}

/// A relation the host declared, by its place among the ones declared with the request.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HostedNumber(pub u32);

/// How a comparison compares its sides.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Test {
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
    /// Numbers written as text, compared as numbers.
    FloatTextGreaterThanOrEqual,
    /// `in`.
    In,
    /// `not in`.
    NotIn,
    /// `glob_any`.
    GlobAny,
    /// `not glob_any`.
    NotGlobAny,
    /// `starts_with`.
    StartsWith,
    /// `ends_with`.
    EndsWith,
    /// `matches`: a regular expression searches the text.
    Matches,
    /// Canonical numeric text or whole numbers, compared without integer rounding.
    NumericGreaterThanOrEqual,
    /// `integer_successor(left, right)`: right is exactly left plus one; integers only.
    IntegerSuccessor,
    /// `natural_lt(left, right)`: case-sensitive natural text order; ASCII digit runs,
    /// fraction-style zero prefixes, Unicode whitespace separators, no normalization.
    NaturalLessThan,
}

/// One condition of a `select` step.
#[derive(Clone, Debug)]
pub struct Comparison {
    /// How the sides compare.
    pub test: Test,
    /// The left side.
    pub left: Operand,
    /// The right side.
    pub right: Operand,
    /// The optional index into the owning plan's source-origin table; diagnostic only.
    pub origin: Option<ComparisonOriginId>,
}

/// A source-origin table index attached to a comparison for diagnostics; meaningful only with its owning plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ComparisonOriginId(pub u32);

/// One side of a comparison.
#[derive(Clone, Debug, PartialEq)]
pub enum Operand {
    /// A column of the step's input.
    Column(ColumnIndex),
    /// A parameter.
    Parameter(ParameterId),
    /// A value the query wrote.
    Literal(QueryLiteral),
    /// A function applied to operands: text made from the row.
    Function(Applied),
}

/// A function a comparison applies to text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Function {
    /// `concat`: the texts one after the other.
    Concat,
    /// `pascal`: `todo_lists` as `TodoLists`.
    Pascal,
    /// `camel`: `todo_lists` as `todoLists`.
    Camel,
    /// `snake`: `TodoLists` as `todo_lists`.
    Snake,
    /// `kebab`: `TodoLists` as `todo-lists`.
    Kebab,
    /// `lower`: Unicode lowercase text, retaining punctuation and whitespace.
    Lower,
}

/// A function and the operands it is applied to.
#[derive(Clone, Debug, PartialEq)]
pub struct Applied {
    /// The function.
    pub function: Function,
    /// Its arguments, in order.
    pub arguments: Vec<Operand>,
}

/// A value the query wrote: text that entered the engine, or a number.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QueryLiteral {
    /// A whole number.
    Number(Integer),
    /// Text.
    Text(LiteralText),
}

/// The heading of a column of the answer.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Heading(pub Box<str>);

/// Which fault a query text was refused for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefusedKind {
    /// The text is not RyoQL.
    Syntax,
    /// The query reads a relation ryo does not own.
    UnknownRelation,
    /// A comparison puts unlike types side by side.
    Type,
    /// The query is wrong in another way.
    Invalid,
    /// The catalog of relations could not be read.
    Catalog,
}

/// What a refusal says: the relation a query named that is not one, or the reason, as the compiler worded it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RefusalMessage(pub String);

/// A query text that was refused: which fault, what it says, and where in the text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RefusedQuery {
    /// Which fault.
    pub kind: RefusedKind,
    /// What it says.
    pub message: RefusalMessage,
    /// Where in the text; empty for a fault that is not in the text.
    pub span: Span,
    /// The original query-module source and import/call path, when the refusal came from a fragment.
    pub origin: Option<crate::domain::program::types::QueryOrigin>,
}

impl std::fmt::Display for RefusedQuery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = &self.message.0;
        match self.kind {
            RefusedKind::Syntax => write!(f, "{message} at byte {}", self.span.start.0),
            RefusedKind::UnknownRelation => write!(f, "no relation named `{message}`"),
            RefusedKind::Type | RefusedKind::Invalid => write!(f, "{message}"),
            RefusedKind::Catalog => write!(f, "the relation catalog failed: {message}"),
        }
    }
}

/// What an operation reads: up to three inputs, as many as the operation names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Inputs {
    /// The inputs in the order the operator reads them; the places past `used` repeat the first.
    pub list: [Input; 3],
    /// How many of them the operation names.
    pub used: InputCount,
}

/// How many inputs an operation reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputCount(pub usize);

impl PartialEq for Comparison {
    fn eq(&self, other: &Self) -> bool {
        self.test == other.test && self.left == other.left && self.right == other.right
    }
}
