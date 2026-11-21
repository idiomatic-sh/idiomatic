//! The types of the `planning` part: what a branch is planned for, the order chosen, and the accesses as both emitters read them.

use crate::domain::lowering::definitions::types::RowEstimate;
use crate::domain::lowering::flattening::types::FlatVariable;
use crate::domain::steps::stages::types::UseMode;
use crate::domain::steps::types::{
    AnchorKinds, AxisDirection, CallMeaning, ColumnMask, DefinitionId, MadeColumn, MadeTableId,
    MatchColumns, ReadColumn, Register, RegisterList, StepOperand, StepOperands, StepRows,
    TestOperation,
};
use ryo_relations::domain::relations::types::{Count, Relation};
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
    pub(crate) order: Vec<AtomPlace>,
    /// The rows the order visits, by the fixed priors.
    pub(crate) cost: RowEstimate,
}

/// One access as written, its registers decided: what the step emitter and the stage emitter each write in their own form.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum WrittenAccess {
    /// Each row of a source whose columns hold the values; no column: every row. `out` is the row's handle.
    Rows {
        /// The source.
        source: StepRows,
        /// The key's columns.
        columns: MatchColumns,
        /// The key's values.
        values: StepOperands,
        /// The kinds a bucket of the node table names, when the vocabulary has them.
        kinds: Option<KindAnchor>,
        /// Whether rows go on, or only whether a row is there: a decision of a source's own rows.
        mode: UseMode,
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
        /// What is walked toward the root.
        edges: NearestWalked,
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
    /// Each element reached over one or more edges of a definition from a seed: a program's only.
    Closure {
        /// What gives the next elements of the one in `from`.
        edges: WalkEdges,
        /// Where the element whose edges are asked is written.
        from: Register,
        /// The element walked from.
        seed: Register,
        /// The element reached.
        out: Register,
    },
    /// A per-row meaning of the operators crate.
    Call {
        /// The meaning.
        meaning: CallMeaning,
        /// What it reads.
        arguments: StepOperands,
        /// Where what it gives goes.
        outputs: RegisterList,
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
    pub(crate) uses: HashMap<FlatVariable, Count>,
    /// The registers given so far.
    pub(crate) environment: Environment,
    /// The accesses written so far.
    pub(crate) written: Vec<WrittenAccess>,
    /// How many atoms could not be written: a term that should have been given was not. A branch with one has no form.
    pub(crate) missed: Count,
}

/// An atom of a branch, by its place among the branch's atoms.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct AtomPlace(pub(crate) usize);

/// An access that can be placed next: the atom, and the rows it is taken to give each time it runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct NextAccess {
    /// The atom.
    pub(crate) atom: AtomPlace,
    /// The rows it gives.
    pub(crate) rows: RowEstimate,
}

/// Whether an atom has its place in the order yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AtomPlacing {
    /// Not yet.
    Waiting,
    /// It has.
    Placed,
}

/// Whether a term is given at a point of a branch's order: a constant, the file, or a variable some access gave.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TermGiven {
    /// It is.
    Given,
    /// A variable nothing gave yet.
    Open,
}

/// The variables given at a point of a branch's order, with the branch's pins.
#[derive(Clone, Copy, Debug)]
pub(crate) struct GivenTerms<'a> {
    /// The variables some access gave.
    pub(crate) variables: &'a std::collections::HashSet<FlatVariable>,
    /// The constant each pinned variable is held to.
    pub(crate) pins:
        &'a HashMap<FlatVariable, crate::domain::lowering::flattening::types::FlatTerm>,
}

/// The state of one completion of a branch's order.
#[derive(Debug)]
pub(crate) struct OrderCompletion<'a> {
    /// The drafts the estimates read.
    pub(crate) drafts: &'a crate::domain::lowering::definitions::types::Drafts,
    /// The branch.
    pub(crate) branch: &'a crate::domain::lowering::flattening::types::FlatBranch,
    /// What the branch is planned for.
    pub(crate) role: UnitRole,
    /// The variables given so far.
    pub(crate) given: std::collections::HashSet<FlatVariable>,
    /// The constant each pinned variable of the branch is held to.
    pub(crate) pins: HashMap<FlatVariable, crate::domain::lowering::flattening::types::FlatTerm>,
    /// Which atoms are placed.
    pub(crate) placed: Vec<AtomPlacing>,
    /// The order so far.
    pub(crate) order: Vec<AtomPlace>,
    /// The rows the unit holds at this point.
    pub(crate) rows: RowEstimate,
    /// The rows visited so far.
    pub(crate) cost: RowEstimate,
    /// Whether rows were enumerated yet.
    pub(crate) enumerated: Enumerated,
}

/// The kinds a bucket of the node table names, with the node relation they are read through.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct KindAnchor {
    /// The node relation.
    pub(crate) relation: Relation,
    /// The kinds.
    pub(crate) kinds: AnchorKinds,
}

/// An atom that waits for its place.
#[derive(Clone, Copy, Debug)]
pub(crate) struct WaitingAtom<'a> {
    /// Its place in the branch.
    pub(crate) place: AtomPlace,
    /// The atom.
    pub(crate) atom: &'a crate::domain::lowering::flattening::types::FlatAtom,
}

/// What a branch is written as: its role, and the definition whose pattern or table it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WrittenUnit {
    /// The role.
    pub(crate) role: UnitRole,
    /// The definition; none for a rule's own branch.
    pub(crate) definition: Option<DefinitionId>,
}

/// What a branch is written from: the registers given at its start, the accesses that test them, and the columns of its
/// row its unit gives (none: every column).
#[derive(Clone, Debug, Default)]
pub(crate) struct BranchStart<'a> {
    /// The registers given at the start.
    pub(crate) environment: Environment,
    /// The accesses written before the branch's own.
    pub(crate) written: Vec<WrittenAccess>,
    /// The columns the unit gives.
    pub(crate) gives: Option<&'a ColumnMask>,
}

/// What a call-form pattern is of: the definition, its formals, and the columns it binds and collects.
#[derive(Clone, Copy, Debug)]
pub(crate) struct PatternColumns<'a> {
    /// The definition.
    pub(crate) definition: DefinitionId,
    /// One register per column of the definition.
    pub(crate) formals: &'a [Register],
    /// The columns that arrive bound.
    pub(crate) bound: &'a ColumnMask,
    /// The columns a row gives back.
    pub(crate) collected: &'a ColumnMask,
}

/// A use of a definition by key, as a walk's body: the columns bound and their cells, and the columns a row gives back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct EdgeUse {
    /// The definition.
    pub(crate) definition: DefinitionId,
    /// The columns bound, ascending.
    pub(crate) bound: ColumnMask,
    /// The columns collected.
    pub(crate) collected: ColumnMask,
    /// The cells of the bound columns.
    pub(crate) values: StepOperands,
}

/// What a closure's walk asks for the next elements.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum WalkEdges {
    /// A use of the edges' definition by key.
    Use(EdgeUse),
    /// Accesses written in place over a source's own rows; a row is the register that holds the next element.
    Inline {
        /// The accesses.
        steps: Vec<WrittenAccess>,
        /// The register a row gives.
        gives: Register,
    },
}

/// What a nearest walk's step climbs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum NearestWalked {
    /// The tree of a node relation.
    Tree(Relation),
    /// Edges asked for the parent of the element in `from`.
    Derived {
        /// What gives the parent.
        edges: WalkEdges,
        /// Where the element whose parent is asked is written.
        from: Register,
    },
}
