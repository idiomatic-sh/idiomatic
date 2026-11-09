//! The types of the `presence` concept: plain data, public fields and derives, with no constructor and no inherent impl.
use ryo_relations::domain::relations::types::{CellKind, Relation};
use ryo_values::domain::plan_parts::types::{ColumnIndex, PlanComparison};

/// A presence test, by its place in the table.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TestId(pub usize);

/// A presence formula, by its place in the table. The first formula is `Always`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FormulaId(pub usize);

/// Whether a file has a row of a relation whose columns, read at one row, pass every comparison. An empty list asks only
/// whether the relation has a row.
#[derive(Clone, Debug, PartialEq)]
pub struct PresenceTest {
    /// The relation read.
    pub relation: Relation,
    /// The comparisons, on the relation's own columns, in the order the step that made them evaluates them.
    pub comparisons: Box<[PlanComparison]>,
    /// The kinds of the relation's columns, which the comparisons read.
    pub kinds: Box<[CellKind]>,
}

/// A necessary condition for a node to have any row in one file: monotone, over presence tests.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum PresenceFormula {
    /// The node may have rows in any file.
    Always,
    /// The file passes this test.
    Test(TestId),
    /// Every formula holds.
    All(Box<[FormulaId]>),
    /// At least one formula holds.
    Any(Box<[FormulaId]>),
}

/// The tests of one relation, which one pass over its rows decides together.
#[derive(Clone, Debug, PartialEq)]
pub struct RelationTests {
    /// The relation.
    pub relation: Relation,
    /// Its tests.
    pub tests: Box<[TestId]>,
}

/// The presence formulas of one request: its tests, its formulas and, for each region of its executable program, the
/// formula that must hold for the region to have a row or a row error in a file.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PresenceTable {
    /// Every test, by its id.
    pub tests: Box<[PresenceTest]>,
    /// Every formula, by its id; a formula's parts come before it.
    pub formulas: Box<[PresenceFormula]>,
    /// The tests grouped by the relation they read.
    pub relations: Box<[RelationTests]>,
    /// The formula of each region, by region: a region whose formula fails in a file publishes the empty table there.
    pub regions: Box<[FormulaId]>,
}

/// The presence formulas of every node of a request graph, by node.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GraphPresence {
    /// The tests and formulas the nodes name; its region table is empty until the program's regions are known.
    pub table: PresenceTable,
    /// The skip formula of each node: its rows' condition, or a row error upstream.
    pub nodes: Box<[FormulaId]>,
}

/// One relation read by one node input: reads of one relation by two inputs are two reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ReadId(pub usize);

/// One appearance of a read's rows in a row of a node: each side of a join is its own, so the two sides of a self join
/// are two occurrences of one read.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct OccurrenceId(pub usize);

/// Where a column comes from: one column of one occurrence of a relation read.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ColumnOrigin {
    /// The read.
    pub read: ReadId,
    /// The occurrence of the read's row.
    pub occurrence: OccurrenceId,
    /// The relation's own column.
    pub column: ColumnIndex,
}

/// Whether a region can have a row or a row error in one file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegionPresence {
    /// It can: it runs.
    Present,
    /// It cannot: it publishes the empty table.
    Absent,
}

/// The hash of a test's relation and comparisons: tests of one bucket are compared in full.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct TestBucket(pub u64);

/// A node of the request graph, by its place in the fold.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct FoldedNode(pub usize);

/// The occurrence a side of a two-input node gives an upstream occurrence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct SideOccurrence {
    /// The two-input node.
    pub node: FoldedNode,
    /// Its side.
    pub side: crate::domain::operation_schema::types::OriginSide,
    /// The occurrence on that side's input.
    pub upstream: OccurrenceId,
}

/// One comparison of a select or join filter, and the origin of the column it reads.
#[derive(Clone, Copy, Debug)]
pub(crate) struct OriginComparison<'c> {
    /// The comparison.
    pub comparison: &'c PlanComparison,
    /// The origin of its left column, when it reads one that has one.
    pub origin: Option<ColumnOrigin>,
}

/// The comparisons of one select or join filter on one occurrence of one read.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ComparisonGroup {
    /// The read.
    pub read: ReadId,
    /// The occurrence.
    pub occurrence: OccurrenceId,
    /// The comparisons, renumbered to the relation's own columns.
    pub comparisons: Vec<PlanComparison>,
}

/// The tests and formulas a fold has made, each kept once, and the reads and occurrences it has numbered.
#[derive(Debug)]
pub(crate) struct PresenceTables {
    /// Every test, by its id.
    pub tests: Vec<PresenceTest>,
    /// The tests of each bucket.
    pub test_buckets: std::collections::HashMap<TestBucket, Vec<TestId>>,
    /// Every formula, by its id.
    pub formulas: Vec<PresenceFormula>,
    /// The id of each formula.
    pub formula_index: std::collections::HashMap<PresenceFormula, FormulaId>,
    /// The relation of each read.
    pub reads: Vec<Relation>,
    /// The occurrence each side of each two-input node gives each upstream occurrence.
    pub occurrences: std::collections::HashMap<SideOccurrence, OccurrenceId>,
    /// The next occurrence to number.
    pub next_occurrence: OccurrenceId,
}

/// Whether a node, and so everything downstream of it, may have rows in any file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Conditionality {
    /// Its rows depend on the file's facts.
    Conditional,
    /// A bound observation, a cross-file step, an invalid schema or unknown reads is upstream.
    Unconditional,
}

/// What the fold knows of one node.
#[derive(Clone, Debug)]
pub(crate) struct NodeFacts {
    /// Its presence formula.
    pub presence: FormulaId,
    /// Its failure term, when an operator upstream can fail on rows.
    pub failure: Option<FormulaId>,
    /// The origin of each of its columns.
    pub origins: Vec<Option<ColumnOrigin>>,
    /// The native tree form its output keeps.
    pub tree: Option<crate::domain::table_cells::types::TreeForm>,
    /// Whether it may have rows in any file.
    pub conditionality: Conditionality,
}

/// Which inputs must have a row in a file for an operation to answer any row there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresenceMask {
    /// Every input the mask names must have a row.
    AllOf(InputMask),
    /// At least one input the mask names must have a row.
    AnyOf(InputMask),
    /// The answer can have rows whatever the inputs hold.
    Always,
}

/// Inputs by their place: the bit of an input is `1 << place`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputMask(pub u8);

/// Which inputs' rows can make an operation fail on what the rows hold, not on their schema.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowFailure {
    /// No row of any input can make it fail.
    Never,
    /// A row of an input the mask names can make it fail.
    OnRows(InputMask),
}
