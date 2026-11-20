//! The types of the `planning` part: what a branch is planned for, the order chosen, and the accesses as both emitters read them.

use crate::domain::lowering::definitions::types::RowEstimate;
use crate::domain::lowering::flattening::types::FlatVariable;
use crate::domain::steps::stages::types::UseMode;
use crate::domain::steps::types::{
    AnchorKinds, AxisDirection, ColumnMask, DefinitionId, MadeColumn, MadeTableId, MatchColumns,
    ReadColumn, Register, RegisterList, StepOperand, StepOperands, StepRows, TestOperation,
};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_orders::types::OrderBound;
use std::collections::HashMap;

/// What a branch is planned and written for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UnitRole {
    /// A rule's own branch: its first enumeration is its anchor, once per file; every access after it is keyed.
    Branch,
    /// A call-form pattern: its bound cells are given, and every access is keyed.
    Pattern,
    /// A pipeline of a table: its first enumeration is its scan, and a later access with no key is a product stage.
    Pipeline,
}

/// The order a branch's atoms are placed in, and what the order is taken to cost.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PlannedBranch {
    /// The atoms, by place in the branch, in the order they run.
    pub(crate) order: Vec<usize>,
    /// The rows the order visits, by the fixed priors.
    pub(crate) cost: RowEstimate,
}

/// One access as written, its registers decided: what the step emitter and the stage emitter each write in their own form.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Emitted {
    /// Each row of a source whose columns hold the values; no column: every row. `out` is the row's handle.
    Rows {
        /// The source.
        source: StepRows,
        /// The key's columns.
        columns: MatchColumns,
        /// The key's values.
        values: StepOperands,
        /// The kinds a bucket of the node table names, when the vocabulary has them.
        kinds: Option<(Relation, AnchorKinds)>,
        /// The row's handle.
        out: Register,
    },
    /// A keyed use of a definition.
    Use {
        /// The definition.
        definition: DefinitionId,
        /// The columns bound.
        bound: ColumnMask,
        /// The columns collected.
        collected: ColumnMask,
        /// The cells of the bound columns.
        values: StepOperands,
        /// Where the collected cells go.
        outputs: RegisterList,
        /// Whether rows go on, or only whether a row is there.
        mode: UseMode,
    },
    /// The rows of an ordered table nearest a bound.
    Latest {
        /// The table.
        table: MadeTableId,
        /// The key's cells.
        key: StepOperands,
        /// The bound.
        bound: StepOperand,
        /// Which side.
        side: OrderBound,
        /// The row's handle.
        out: Register,
    },
    /// A column of a row.
    Read {
        /// The source.
        source: StepRows,
        /// The column.
        column: ReadColumn,
        /// The row's handle.
        row: Register,
        /// Where the cell goes.
        out: Register,
    },
    /// A test.
    Test {
        /// The comparison.
        test: TestOperation,
        /// Its left operand.
        left: StepOperand,
        /// Its right operand.
        right: StepOperand,
    },
    /// Each element along a tree axis from an element.
    Axis {
        /// The node relation whose tree is walked.
        relation: Relation,
        /// The axis.
        direction: AxisDirection,
        /// The element walked from.
        start: Register,
        /// The element reached.
        out: Register,
    },
    /// Each child of an element.
    Children {
        /// The node relation whose tree is walked.
        relation: Relation,
        /// The parent.
        parent: Register,
        /// The child.
        out: Register,
    },
    /// The test that one element is a proper ancestor of another: a program's only.
    Within {
        /// The node relation whose tree is walked.
        relation: Relation,
        /// The ancestor.
        ancestor: Register,
        /// The descendant.
        descendant: Register,
    },
    /// The nearest ancestor of an element that a definition holds in a column: a program's only.
    Nearest {
        /// The node relation whose tree is walked.
        relation: Relation,
        /// The definition of the domain.
        domain: DefinitionId,
        /// The domain's column.
        column: MadeColumn,
        /// Where the element under test is written.
        element: Register,
        /// The element walked from.
        start: Register,
        /// The ancestor reached.
        out: Register,
    },
    /// A register takes a value: a program's only.
    Let {
        /// The value.
        from: StepOperand,
        /// The register.
        out: Register,
    },
}

/// A cell of a row already read: where a pipeline reads the file's own cell, or a pinned variable's, from when a key needs it
/// in a register.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FileCell {
    /// The row's source.
    pub(crate) source: StepRows,
    /// The cell's column.
    pub(crate) column: ReadColumn,
    /// The row's handle.
    pub(crate) row: Register,
}

/// Whether a unit has enumerated rows yet: its first enumeration may be file-sized, in a branch and in a pipeline.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Enumerated {
    /// Not yet.
    #[default]
    NotYet,
    /// It has.
    Already,
}

/// What is known while a branch is written: the register of each variable given so far, and the pinned variables an access
/// was keyed by, each with the row its cell can be read from when a later key needs it in a register.
#[derive(Clone, Debug, Default)]
pub(crate) struct Environment {
    /// The register of each variable some access gave.
    pub(crate) registers: HashMap<FlatVariable, Register>,
    /// The constant each pinned variable of the branch is held to.
    pub(crate) pins: HashMap<FlatVariable, crate::domain::lowering::flattening::types::FlatTerm>,
    /// The pinned variables an access was keyed by: given, as their constant; with the cell of the row found, when there is one.
    pub(crate) keyed: HashMap<FlatVariable, Option<FileCell>>,
    /// A file column already at hand.
    pub(crate) file: Option<FileCell>,
    /// Whether rows were enumerated yet.
    pub(crate) enumerated: Enumerated,
}

/// What the emitter of one branch holds: the drafts it asks patterns of and names registers in, and the branch's own facts.
#[derive(Debug)]
pub(crate) struct Emitter<'a> {
    /// The pack's drafts.
    pub(crate) drafts: &'a mut crate::domain::lowering::definitions::types::Drafts,
    /// The language's kinds.
    pub(crate) kinds: &'a ryo_values::domain::descriptions::types::KindVocabulary,
    /// What the branch is written for.
    pub(crate) role: UnitRole,
    /// The definition whose pattern or table the branch is written for; none for a rule's own branch.
    pub(crate) unit: Option<DefinitionId>,
    /// The branch's row.
    pub(crate) row: Vec<crate::domain::lowering::flattening::types::FlatTerm>,
    /// How often each variable is written in the branch and its row.
    pub(crate) uses: HashMap<FlatVariable, usize>,
    /// The registers given so far.
    pub(crate) environment: Environment,
    /// The accesses written so far.
    pub(crate) written: Vec<Emitted>,
    /// How many atoms could not be written: a term that should have been given was not. A branch with one has no form.
    pub(crate) missed: usize,
}
