//! The types of the `plan_parts` concept: the numbers, text and comparisons a query plan is made of. Plain data, public fields and derives, with no constructor and no inherent impl.

use ryo_relations::domain::relations::types::{Integer, Relation, Span};

/// The text of a RyoQL query.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QueryText(pub Box<str>);

/// Text the query wrote: a string, a path or a glob.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub struct LiteralText(pub Box<str>);

/// A parameter's name, as the host supplies it.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub struct ParameterName(pub Box<str>);

/// A key of the tables a `list<table>` parameter holds, naming the column its values fill: text from the query.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub struct TableKey(pub Box<str>);

/// A pattern with `{slot}` where text is captured.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub struct CapturePattern(pub Box<str>);

/// Whether a projection drops the rows that repeat.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum RepeatedRows {
    /// A row that repeats is kept once.
    Dropped,
    /// Every row is kept: no row repeats, because a column kept says which row it is.
    Kept,
}

/// An input of a plan's frontier, by its place among them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InputNumber(pub usize);

/// An observation binding, by its place among the bindings of a request.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BindingPlace(pub usize);

/// A step of the plan, by its place in the plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub struct StepId(pub u32);

/// A column, by its place in the table that holds it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub struct ColumnIndex(pub u32);

/// A parameter, by its place among the plan's parameters.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Input {
    /// A relation ryo owns.
    Relation(Relation),
    /// An earlier step.
    Step(StepId),
    /// An explicit typed producer binding, resolved by the request graph rather than the relation catalog.
    BoundObservation(ObservationBindingId),
}

/// Which side of a join a column comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Side {
    /// The left input.
    Left,
    /// The right input.
    Right,
}

/// Which way an axis walks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct JoinColumn {
    /// The input it comes from.
    pub side: Side,
    /// Its place in that input.
    pub column: ColumnIndex,
}

/// Physical source-keyed lookup of a positive transitive closure, internal to a compiled join.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub struct HostedNumber(pub u32);

/// How a comparison compares its sides.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
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
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PlanComparison {
    /// How the sides compare.
    pub test: Test,
    /// The left side.
    pub left: Operand,
    /// The right side.
    pub right: Operand,
    /// The optional index into the owning plan's source-origin table; diagnostic only.
    pub origin: ComparisonOrigin,
}

/// Whether a comparison carries a source-origin table index.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ComparisonOrigin {
    /// The comparison has an entry in the owning plan's source-origin table.
    Recorded(ComparisonOriginId),
    /// The comparison has none.
    Unrecorded,
}

/// A source-origin table index attached to a comparison for diagnostics; meaningful only with its owning plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ComparisonOriginId(pub u32);

/// One side of a comparison.
#[derive(Clone, Debug, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
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
#[derive(Clone, Debug, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Applied {
    /// The function.
    pub function: Function,
    /// Its arguments, in order.
    pub arguments: Vec<Operand>,
}

/// A value the query wrote: text that entered the engine, or a number.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum QueryLiteral {
    /// A whole number.
    Number(Integer),
    /// Text.
    Text(LiteralText),
}

/// The heading of a column of the answer.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Heading(pub Box<str>);

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

/// Hashes what equality compares; the diagnostic origin is not part of either.
impl std::hash::Hash for PlanComparison {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.test.hash(state);
        self.left.hash(state);
        self.right.hash(state);
    }
}

impl PartialEq for PlanComparison {
    fn eq(&self, other: &Self) -> bool {
        self.test == other.test && self.left == other.left && self.right == other.right
    }
}

/// A bound input's stable local position within one compiled consumer plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub struct ObservationBindingId(pub u32);

/// Where a step runs: in the worker that has the file, or once after the workers on the rows they
/// handed over.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum StepScope {
    /// On one file's rows, in the worker.
    PerFile,
    /// On the frontier rows of every file.
    CrossFile,
}
