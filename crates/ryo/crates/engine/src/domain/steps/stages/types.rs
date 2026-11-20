//! The types of the `stages` part of the `steps` concept: a fill's pipeline and its stages, one variant per stage, with its
//! operands. No stage holds a step program: a table's rows are made by compiled loops over the batch's columns. Crate-private.

use crate::domain::steps::types::{
    AxisDirection, CallMeaning, ColumnMask, DefinitionId, MadeTableId, MatchColumns, PlanPoint,
    RangeBound, ReadColumn, Register, RegisterList, StepOperand, StepOperands, StepRows,
    TestOperation,
};
use ryo_language::domain::node_table::types::FieldCode;
use ryo_language::domain::semantics::types::RenderContext;
use ryo_values::domain::plan_orders::types::OrderBound;

/// One pipeline of a fill: stages over batches of rows, then the row written.
#[derive(Debug)]
pub(crate) struct Pipeline {
    /// The stages; the first is a `Scan`.
    pub(crate) stages: Box<[PlacedStage]>,
    /// One operand per column of the table, read after the last stage.
    pub(crate) row: StepOperands,
}

/// A stage and the plan point it is counted at.
#[derive(Debug)]
pub(crate) struct PlacedStage {
    /// Its place among its unit's plan points.
    pub(crate) point: Option<PlanPoint>,
    /// The stage.
    pub(crate) stage: FillStage,
}

/// A stage of a pipeline: each takes a batch of rows and gives a batch; a register is a column of the batch.
#[derive(Debug)]
pub(crate) enum FillStage {
    /// The rows of a source, or of one constant bucket of it.
    Scan {
        source: StepRows,
        columns: MatchColumns,
        values: StepOperands,
        out: Register,
    },
    /// Per row, the rows of a source whose columns hold the values. In rows mode a register stands among the values or in both bounds of the range;
    /// with none it would be every row of a bucket per row, which is a product and is written as `Product`.
    Probe {
        source: StepRows,
        columns: MatchColumns,
        values: StepOperands,
        range: Option<ProbeRange>,
        mode: UseMode,
        out: Register,
    },
    /// Per row, every row of a source: the pairs of a join the plan states with no key within the file.
    Product { source: StepRows, out: Register },
    /// Per row, a use of a definition: a probe of its table, or its call-form program once per distinct key.
    Use {
        definition: DefinitionId,
        bound: ColumnMask,
        collected: ColumnMask,
        values: StepOperands,
        mode: UseMode,
        outputs: RegisterList,
    },
    /// Per row, the rows of an ordered table nearest a bound.
    Latest {
        table: MadeTableId,
        key: StepOperands,
        bound: StepOperand,
        side: OrderBound,
        out: Register,
    },
    /// Per row, a column of a row by its handle.
    Read {
        source: StepRows,
        column: ReadColumn,
        row: Register,
        out: Register,
    },
    /// Keeps the rows a test holds of.
    Keep {
        test: TestOperation,
        left: StepOperand,
        right: StepOperand,
    },
    /// Per row, a per-row meaning of the operators crate.
    Call {
        meaning: CallMeaning,
        arguments: StepOperands,
        outputs: RegisterList,
    },
    /// Per row, the source form of a node in a syntax role.
    Render {
        node: Register,
        context: RenderContext,
        out: Register,
    },
    /// Per row, the elements a tree walk reaches from a node.
    Walk {
        walk: TreeWalk,
        start: Register,
        out: Register,
    },
}

/// The tree walks a pipeline makes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TreeWalk {
    /// Each child, of one field when given.
    Children(Option<FieldCode>),
    /// Each element along an axis.
    Axis(AxisDirection),
}

/// The range a probe is held to, on one ordered column of its source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ProbeRange {
    /// The ordered column.
    pub(crate) column: ReadColumn,
    /// The lower end.
    pub(crate) low: RangeBound,
    /// The upper end.
    pub(crate) high: RangeBound,
}

/// What a probe or a use does with what it finds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UseMode {
    /// One row goes on per row found.
    Rows,
    /// The row goes on when a row is found.
    Present,
    /// The row goes on when none is.
    Absent,
}
