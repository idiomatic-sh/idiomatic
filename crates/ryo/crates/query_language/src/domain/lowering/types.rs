//! The types of the `lowering` concept: plain data, public fields and derives, with no
//! constructor and no inherent impl.

use ryo_relations::domain::relations::types::{CellKind, Relation};
use ryo_relations::domain::terms::types::Vocabulary;

use ryo_values::domain::hosted::types as hosted;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Comparison;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plan_parts::types::JoinColumn;
use ryo_values::domain::plan_parts::types::Operand;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plans::types as plans;
use ryo_values::domain::plans::types::ParameterType;
use ryo_values::domain::plans::types::StepScope;
use ryo_values::domain::program::types::{
    AliasName, ColumnName, ParameterDeclaration, QueryOrigin, QuerySpan, StepName,
};

/// A column of a step's answer while the plan is built: the name the query gives it and what it holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PlannedColumn {
    /// The name the query writes for it: text that entered the engine.
    pub name: ColumnName,
    /// What it holds.
    pub kind: CellKind,
    /// Whether it says which row a row is: no two rows hold the same value.
    pub unique: Uniqueness,
    /// The words it holds: one of a closed set, or any.
    pub words: Vocabulary,
}

/// The columns a `where` can name: those of the rows it filters by their names, and after a join each side's by its alias.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Addressable<'a> {
    /// The columns of the rows filtered, in order.
    pub fields: &'a [PlannedColumn],
    /// The columns the `where` names as `alias.column` after a join, each with its place among `fields`; empty with no join.
    pub sides: &'a [SideColumn],
}

/// A column of one side of a join that a `where` names as `alias.column`, and its place in the rows of the join.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SideColumn {
    /// The alias of the side.
    pub alias: AliasName,
    /// The column of the side.
    pub column: ColumnName,
    /// Its place in the rows of the join.
    pub index: ColumnIndex,
}

/// A step lowered so far: its place in the plan, the name the `let` gave it and its columns.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LoweredStep {
    /// The name the `let` gave it, if the query names it.
    pub name: StepLabel,
    /// Its columns, in order.
    pub fields: Vec<PlannedColumn>,
    /// Where it runs.
    pub scope: StepScope,
}

/// Whether the `let` gave a step a name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum StepLabel {
    /// The query does not name this step.
    Unnamed,
    /// The `let` gave it this name.
    Named(StepName),
}

/// Where the rows an input provides are made, and the columns they have.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ScopedInput {
    /// Where they are made.
    pub scope: StepScope,
    /// Their columns, in order.
    pub fields: Vec<PlannedColumn>,
}

/// How many comparison origins a plan has interned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct OriginCount(pub usize);

/// What a source reads, and the columns it has.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Sourced {
    /// What it reads.
    pub input: Input,
    /// Its columns, in order.
    pub fields: Vec<PlannedColumn>,
}

/// A join lowered: the source its rows make, and the columns of either side its `where` names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Joined {
    /// The rows of the join, and their columns.
    pub sourced: Sourced,
    /// The columns of either side the `where` names as `alias.column`.
    pub sides: Vec<SideColumn>,
}

/// How many columns of a step's rows a `select` shows: the first ones; the columns after them only a `where` reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ShownColumns(pub usize);

/// The columns a widening join keeps, and the fields they make.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Widened {
    /// The columns the join gives.
    pub output: Vec<JoinColumn>,
    /// The fields of those columns.
    pub fields: Vec<PlannedColumn>,
}

/// What a comparison's side is, as far as the query says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Typed {
    /// A column of this kind.
    Column(CellKind),
    /// A number the query wrote.
    Number,
    /// Text the query wrote.
    Text,
    /// A parameter of this kind.
    Parameter(ParameterType),
}

/// One side of a comparison, resolved, with what the query says it is.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ResolvedOperand {
    /// The side, as the plan holds it.
    pub operand: Operand,
    /// What the query says it is.
    pub typed: Typed,
}

/// Whether the join of a select is the step the `let` names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LastStep {
    /// The join is the last step of the select, so it is named by the `let`.
    Join,
    /// A filter or a projection follows the join.
    Later,
}

/// Whether a column says which row a row is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Uniqueness {
    /// No two rows hold the same value in it.
    Unique,
    /// Rows can hold the same value in it.
    Shared,
}

/// Whether two sides of a comparison can be compared by its test.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Compatibility {
    /// The test accepts these two types.
    Comparable,
    /// The test does not accept these two types.
    Mismatched,
}

/// Whether an item answers the question a search asks of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SearchMatch {
    /// It does.
    Matches,
    /// It does not.
    Differs,
}

/// Whether a row's columns say which file the row came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FileOrigin {
    /// A column is a file, a node or a span.
    Stated,
    /// No column is.
    Unstated,
}

/// How the pairs of the same-sets idiom were pushed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SameSetsPush {
    /// As the functions met on their name and their digest.
    Joined,
    /// As the digests of every pair of functions.
    Separate,
}

/// The step a relation the host declared was made into.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct HostedStep {
    /// The name a rule writes for it.
    pub name: hosted::HostedName,
    /// The step that makes its rows.
    pub step: StepId,
}

/// The plan under construction.
#[derive(Debug)]
pub(crate) struct Builder<'a> {
    /// The relations the query may read.
    pub catalog: &'a [Relation],
    /// The steps the relations the host declared were made into, by the name a rule writes.
    pub hosted_steps: Vec<HostedStep>,
    /// Typed producer input schemas; these are not Hosted static tables.
    pub observation_inputs: &'a [ryo_values::domain::observations::types::BoundObservationInput],
    /// The parameters the query declares.
    pub parameters: &'a [ParameterDeclaration],
    /// What each parameter holds, in the order declared.
    pub kinds: Vec<ParameterType>,
    /// The steps made so far.
    pub steps: Vec<plans::PlanStep>,
    /// The same steps with the names and columns lowering keeps for the steps below.
    pub lowered: Vec<LoweredStep>,
    /// Where the line being lowered is written.
    pub span: QuerySpan,
    /// The source document and trace of the step currently lowered.
    pub origin: Option<QueryOrigin>,
    /// Source origins interned for comparisons in this plan.
    pub comparison_origins: Vec<QueryOrigin>,
}

/// The join the pairs of the same-sets idiom come from, as recognized.
#[derive(Clone, Debug)]
pub(crate) struct SameSetsJoin {
    /// The step of that join.
    pub met: StepId,
    /// The filter of the pairs over it, when there is one.
    pub narrowing: PairFiltering,
    /// The step both sides of the join read.
    pub left: StepId,
    /// The column of the function in the large step.
    pub first: ColumnIndex,
    /// The columns the join meets on, on the left.
    pub left_on: Vec<ColumnIndex>,
    /// The columns the join meets on, on the right.
    pub right_on: Vec<ColumnIndex>,
    /// The columns the join gives.
    pub output: Vec<JoinColumn>,
}

/// Whether a filter of the pairs follows the join they come from.
#[derive(Clone, Debug)]
pub(crate) enum PairFiltering {
    /// The pairs are the join's rows.
    Unfiltered,
    /// A filter keeps some of the join's rows.
    Filtered(Vec<Comparison>),
}

/// Why an argument cannot be used.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ArgumentFault {
    /// Root and edge entity kinds must agree.
    MatchingGraphEntities,
    /// Matching root/edge partition names must have identical kinds.
    MatchingGraphKeys,
    /// Outgoing edge order must be numeric.
    NumericGraphOrder,
    /// The call leaves it out.
    Missing,
    /// It should name one thing.
    ExpectedName,
    /// It should be a list of names.
    ExpectedNames,
    /// It should name a relation or a step.
    ExpectedInput,
    /// It should be a map.
    ExpectedMap,
    /// It should hold a side and a column.
    ExpectedSideAndColumn,
    /// It should be left or right.
    ExpectedSide,
    /// The two inputs should have the same columns.
    SameColumns,
    /// The two lists of keys should be as long, and not empty.
    MatchingKeys,
    /// It should be a direction.
    ExpectedDirection,
    /// A sibling is walked over the syntax tree's own edges, which keep the children's order.
    SiblingOverTree,
    /// It names a parameter nobody declared.
    Undeclared,
    /// Only numbers and flags can extend a table.
    NumbersAndFlagsOnly,
    /// Only numbers and parameters can extend a table.
    NumbersAndParametersOnly,
    /// It should be a column of text.
    ExpectedTextColumn,
    /// It should be a parameter that holds a list of tables.
    ExpectedTables,
    /// It should be a parameter that holds a list of sets of text.
    ExpectedTextSets,
    /// The operation is not one the language has.
    NoSuchOperation,
    /// A select is written as a select.
    NotACall,
}
