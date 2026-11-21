//! The types of the `steps` concept: the instruction set the lowering writes, one variant per instruction, with its operands; and the stages of a fill's pipeline. Crate-private.

use ryo_language::domain::facts::types::CellWord;
use ryo_language::domain::node_table::types::FieldCode;
use ryo_language::domain::semantics::types::RenderContext;
use ryo_relations::domain::relations::types::{Column, KindBits, NodeColumn, Relation};
use ryo_values::domain::plan_orders::types::OrderBound;
use ryo_values::domain::plan_parts::types::InputNumber;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;

/// A program: steps over registers, run once per candidate.
#[derive(Debug, Default)]
pub(crate) struct StepProgram(pub(crate) Box<[PlacedStep]>);

/// A step and the plan point it is counted at.
#[derive(Debug)]
pub(crate) struct PlacedStep {
    /// Its place among its unit's plan points; none until the numbering pass, and none for a step a later pass adds.
    pub(crate) point: Option<PlanPoint>,
    /// The step.
    pub(crate) step: ProgramStep,
}

/// A place among a unit's plan points, in the pre-order of its program after the structural passes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct PlanPoint(pub(crate) u32);

/// The instruction set the lowering writes: one variant per instruction.
#[derive(Debug)]
pub(crate) enum ProgramStep {
    /// Each id of the kinds, in id order, among the node rows `relation` covers. `relation` is of the node-table form; its
    /// extent (the catalog's `node_extent`) says which rows those are: the file's own tree alone, the rows of embedded
    /// segments left out, or every row of the table.
    EachKind {
        relation: Relation,
        kinds: AnchorKinds,
        out: Register,
    },
    /// Each row of a source whose columns hold the values (a constant: a bucket; a register: a key). `out` is the row's handle.
    EachMatch {
        source: StepRows,
        columns: MatchColumns,
        values: StepOperands,
        out: Register,
    },
    /// Each row of a bucket whose range column lies between the bounds, in that column's order.
    EachRange {
        source: StepRows,
        columns: MatchColumns,
        values: StepOperands,
        range: ReadColumn,
        low: Option<RangeBound>,
        high: Option<RangeBound>,
        out: Register,
        range_out: Option<Register>,
    },
    /// Each row of a source.
    EachRow { source: StepRows, out: Register },
    /// Each child of a node, of one field when given, among the node rows `relation` covers (as `EachKind`'s).
    Children {
        relation: Relation,
        parent: Register,
        field: Option<FieldCode>,
        out: Register,
    },
    /// Each element along a tree axis from a node, among the node rows `relation` covers; never the start itself.
    Axis {
        relation: Relation,
        direction: AxisDirection,
        start: Register,
        out: Register,
    },
    /// The test that one node is a proper ancestor of another, in the tree of the node rows `relation` covers.
    Within {
        relation: Relation,
        ancestor: Register,
        descendant: Register,
    },
    /// The nearest element from a start (never the start) that the condition holds of; the element under test is in `element`.
    Nearest {
        edges: NearestEdges,
        condition: StepBody,
        element: Register,
        start: Register,
        out: Register,
        memo: StepMemo,
    },
    /// Each element reached over one or more edges from a seed; the element whose edges are asked is in `from`.
    Closure {
        edges: StepBody,
        from: Register,
        seed: Register,
        out: Register,
    },
    /// A column of a row, by the row's handle.
    Read {
        source: StepRows,
        column: ReadColumn,
        row: Register,
        out: Register,
    },
    /// A test of two operands.
    Test {
        test: TestOperation,
        left: StepOperand,
        right: StepOperand,
    },
    /// A per-row meaning of the operators crate.
    Call {
        meaning: CallMeaning,
        arguments: StepOperands,
        outputs: RegisterList,
    },
    /// The source form of a node in a syntax role.
    Render {
        node: Register,
        context: RenderContext,
        out: Register,
    },
    /// A register takes a value.
    Let { from: StepOperand, out: Register },
    /// A body must have, or must not have, a row.
    Exists {
        polarity: Polarity,
        body: StepBody,
        memo: StepMemo,
    },
    /// Each distinct row of the bodies goes on, its cells in `outputs`.
    Or {
        bodies: Box<[StepBody]>,
        outputs: RegisterList,
        memo: StepMemo,
    },
    /// Each slot's program runs from here.
    Fork { slots: Box<[StepProgram]> },
    /// A body's row.
    Yield { row: RegisterList },
    /// The rows of an ordered table nearest a bound in a key group: every row tied for nearest. `out` is the row's handle.
    Latest {
        table: MadeTableId,
        key: StepOperands,
        bound: StepOperand,
        side: OrderBound,
        out: Register,
    },
    /// Hands a row over.
    Emit {
        target: EmitTarget,
        columns: StepOperands,
    },
}

/// What a control step runs.
#[derive(Debug)]
pub(crate) enum StepBody {
    /// Steps written in place; each row ends in `Yield`.
    Inline(StepProgram),
    /// A keyed use of a definition: `values` are the cells of the bound columns, ascending; a row is the collected columns, ascending. With rows collected it binds at least one column; with nothing bound it collects nothing and asks only whether a row exists.
    Definition {
        definition: DefinitionId,
        bound: ColumnMask,
        collected: ColumnMask,
        values: StepOperands,
    },
}

/// Whether the answer of a step over an inline body is kept for the file, and by what.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) enum StepMemo {
    /// Run every time.
    #[default]
    None,
    /// Kept by the values of the registers; none: once per file.
    Keyed(RegisterList),
}

/// Where rows are read from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StepRows {
    /// The node table, through a relation of the node-table form. The rows read are those of the relation's extent (the
    /// catalog's `node_extent`): the file's own tree alone, or the rows of embedded segments too.
    Node(Relation),
    /// A filled relation.
    Filled(Relation),
    /// A table of an operation, made in the file: group, ordered or graph.
    Made(MadeTableId),
    /// A table of rows written in the query or bound to a parameter: the same for every file.
    Written(MadeTableId),
    /// Every row of a definition, nothing bound: its table. Only in `EachRow` at a rule branch's start, in `Scan` and in `Product`, and in the `Read` of such a row's columns.
    Definition(DefinitionId),
}

/// A column of a source; its variant is its source's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReadColumn {
    /// A column of the node table.
    Node(NodeColumn),
    /// A column of a filled relation.
    Filled(Column),
    /// A column of a made table, by place.
    Made(MadeColumn),
}

/// The columns a match is on, in an instruction's order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct MatchColumns(pub(crate) Box<[ReadColumn]>);

/// One end of a range.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RangeBound {
    /// The bound's value.
    pub(crate) value: StepOperand,
    /// Whether a row at the bound is in the range.
    pub(crate) end: BoundKind,
}

/// Whether a range holds its bound.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BoundKind {
    /// The bound is in the range.
    Inclusive,
    /// The bound is not.
    Strict,
}

/// What `Nearest` walks.
#[derive(Debug)]
pub(crate) enum NearestEdges {
    /// The tree of the node rows a relation of the node-table form covers, toward the root.
    Tree(Relation),
    /// Edges a body yields: the element whose edges are asked is in `from`, a row is the next element.
    Derived { edges: StepBody, from: Register },
}

/// What bounds the work of a step or a stage each time it runs: a property of the step alone.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum CostClass {
    /// A fixed amount: a read, a test, a parent step.
    Constant,
    /// The rows one key reaches: a node's children, a probe by key, a walk from a node.
    Fanout,
    /// The rows of one group, sorted once.
    Group,
    /// The rows of the file: a kind bucket, a relation or a table read without a key.
    File,
}

/// A register: a cell in a program, a column of the batch in a pipeline.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Register(pub(crate) u32);
/// Registers, in an instruction's order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct RegisterList(pub(crate) Box<[Register]>);
/// How many registers a pack's programs name.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct RegisterCount(pub(crate) u32);
/// A definition, by number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct DefinitionId(pub(crate) u32);
/// A made table, by number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct MadeTableId(pub(crate) u32);
/// A column of a made table or a definition, by place.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct MadeColumn(pub(crate) u16);
/// Columns of a made table, in an instruction's order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ColumnList(pub(crate) Box<[MadeColumn]>);
/// Columns of a definition, ascending and each once: the columns a use binds, or collects. It has no width of its own.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ColumnMask(pub(crate) Box<[MadeColumn]>);
/// The kinds an anchor iterates.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct AnchorKinds(pub(crate) Box<[KindBits]>);
/// A regular expression, by number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct RegexId(pub(crate) u32);
/// The source of a regular expression.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StepRegex(pub(crate) Box<str>);
/// A constant text of the programs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ConstantText(pub(crate) Box<str>);
/// StepOperands, in an instruction's order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct StepOperands(pub(crate) Box<[StepOperand]>);

/// An operand of a step.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum StepOperand {
    /// A register's value.
    Register(Register),
    /// A constant: a base word or a number's bits.
    Word(CellWord),
    /// A set of constants.
    Words(Box<[CellWord]>),
    /// The scanned file.
    File,
}

/// The comparisons a `Test` makes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TestOperation {
    /// Equal.
    Equal,
    /// Not equal.
    NotEqual,
    /// Less than.
    Less,
    /// Less than or equal.
    LessOrEqual,
    /// Greater than.
    Greater,
    /// Greater than or equal.
    GreaterOrEqual,
    /// In a set of constants.
    In,
    /// Not in a set of constants.
    NotIn,
    /// Matches a regular expression.
    Matches(RegexId),
    /// Matches a glob.
    Glob,
    /// Does not match a glob.
    NotGlob,
    /// Starts with.
    StartsWith,
    /// Ends with.
    EndsWith,
    /// Numbers and canonical numeric text, at least.
    NumericAtLeast,
    /// Natural text order.
    NaturalLess,
    /// The right is the left plus one.
    Successor,
}

/// The tree axes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AxisDirection {
    /// The parent.
    Parent,
    /// Each ancestor, nearest first.
    Ancestor,
    /// Each descendant node, in pre-order.
    Descendant,
    /// The next sibling.
    NextSibling,
    /// The previous sibling.
    PreviousSibling,
}

/// The per-row meanings a `Call` runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CallMeaning {
    /// `span-between`.
    SpanBetween,
    /// `span-bounds`.
    SpanBounds,
    /// `source-matches`.
    SourceMatches(RegexId),
    /// `capture`.
    Capture,
    /// `replace`.
    Replace(RegexId),
    /// `words`.
    Words,
    /// `rank`.
    Rank,
    /// `concat`.
    Concat,
    /// `lower`.
    Lower,
    /// `fits-rendered`.
    FitsRendered,
}

/// Whether an `Exists` asks for a row or for none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Polarity {
    /// A row.
    Present,
    /// No row.
    Absent,
}

/// Whose row an `Emit` hands over.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EmitTarget {
    /// A finding of the rule.
    Rule(SelectedRuleOrdinal),
    /// A row of the rule's frontier entry.
    Frontier(SelectedRuleOrdinal, InputNumber),
}
