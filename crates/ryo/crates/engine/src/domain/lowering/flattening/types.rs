//! The types of the `flattening` part of the lowering: a plan step as conjunctive branches of atoms over terms. A branch is
//! the unit the planning orders and the two emitters write: steps for a program, stages for a table's pipeline.

use crate::domain::steps::types::{
    AxisDirection, CallMeaning, DefinitionId, MadeColumn, MadeTableId, Polarity, TestOperation,
};
use ryo_language::domain::semantics::types::RenderContext;
use ryo_relations::domain::relations::types::{Integer, Relation};
use ryo_values::domain::plan_orders::types::OrderBound;

/// What a per-row meaning is: one of the operators crate's, or the language's source form of a node in a syntax role (one
/// node read, its text given, no row when the node has none in the role).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RowMeaning {
    /// A meaning of the operators crate.
    Call(CallMeaning),
    /// The language's rendering of a node in a syntax role.
    Render(RenderContext),
}

/// A variable of a branch: a cell some atom gives and others read.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct FlatVariable(pub(crate) u32);

/// A constant text, by its place among the pack's texts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct TextNumber(pub(crate) u32);

/// What a cell of an atom or a column of a branch's row holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum FlatTerm {
    /// A variable.
    Variable(FlatVariable),
    /// A constant text.
    Text(TextNumber),
    /// A constant number.
    Number(Integer),
    /// The scanned file: every cell of a file column of a per-file step. It is never a key within the file.
    File,
}

/// What an atom's rows are read from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum AtomRows {
    /// A relation of the catalog: the node table through it, or its filled rows.
    Relation(Relation),
    /// A definition: a sub-plan lowered once.
    Definition(DefinitionId),
    /// A group table.
    Made(MadeTableId),
}

/// Whether a source holds at most one row in a file whatever is asked of it: a group table keyed on the file alone.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum RowsPerFile {
    /// At most one.
    One,
    /// Any number.
    Many,
}

/// One conjunct of a branch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum FlatAtom {
    /// A row of a source: one term per column of it.
    Rows {
        /// The source.
        rows: AtomRows,
        /// The row's cells, in the source's column order.
        cells: Vec<FlatTerm>,
        /// Whether the source holds one row a file.
        per_file: RowsPerFile,
    },
    /// The rows of an ordered table nearest a bound in a key group: the `as-of` operation.
    Latest {
        /// The ordered table.
        table: MadeTableId,
        /// The key's cells, the file's left out.
        key: Vec<FlatTerm>,
        /// The bound.
        bound: FlatTerm,
        /// Which side of the bound, strictly or not.
        side: OrderBound,
        /// The cells of the row found, in the table's column order.
        cells: Vec<FlatTerm>,
    },
    /// A source must have, or must not have, a row holding the given cells: a definition, or a relation's own rows.
    Decided {
        /// Which.
        polarity: Polarity,
        /// The source.
        rows: AtomRows,
        /// The cell asked of each column; none: any.
        cells: Vec<Option<FlatTerm>>,
    },
    /// One element reached from another along a tree axis of a node relation.
    Walk {
        /// The node relation whose tree is walked.
        relation: Relation,
        /// The axis, from the start.
        direction: AxisDirection,
        /// The element walked from.
        start: FlatTerm,
        /// The element reached.
        reached: FlatTerm,
    },
    /// The nearest ancestor of an element that a definition holds in a column: the domain of a nearest axis.
    Nearest {
        /// What is walked toward the root.
        over: NearestOver,
        /// What holds the elements a walk may stop at.
        domain: NearestDomain,
        /// The element walked from.
        start: FlatTerm,
        /// The ancestor reached.
        reached: FlatTerm,
    },
    /// One element reached from another over one or more edges of a definition: the `closure` operation.
    Closure {
        /// The edges.
        edges: EdgeColumns,
        /// The cells of the edges' key columns, in the keys' order: the graph the walk stays in.
        keys: Vec<FlatTerm>,
        /// The element walked from.
        source: FlatTerm,
        /// The element reached.
        reached: FlatTerm,
    },
    /// A per-row meaning: it reads its arguments and gives its outputs, or gives no row.
    Called {
        /// The meaning.
        meaning: RowMeaning,
        /// What it reads.
        arguments: Vec<FlatTerm>,
        /// What it gives.
        outputs: Vec<FlatTerm>,
    },
    /// A test of two terms.
    Test {
        /// The comparison.
        test: TestOperation,
        /// Its left term.
        left: FlatTerm,
        /// Its right term.
        right: FlatTerm,
    },
}

/// One conjunctive branch: its atoms all hold, and its row is what it gives.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct FlatBranch {
    /// The atoms.
    pub(crate) atoms: Vec<FlatAtom>,
    /// One term per column of the step.
    pub(crate) row: Vec<FlatTerm>,
}

/// Whether a step's branches hold a walk that no stage runs: a nearest walk or a closure. A pipeline runs such a walk
/// through a pattern of the step's own definition, which names the walk's ends by their columns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OwnWalks {
    /// Some branch holds one.
    Walked,
    /// None does.
    Stageable,
}

/// A column of a row, or a cell of an atom, by place.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct RowColumn(pub(crate) usize);

/// Two terms an equality holds equal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct EqualTerms {
    /// One term.
    pub(crate) left: FlatTerm,
    /// The other.
    pub(crate) right: FlatTerm,
}

/// The edges a closure walks: a definition, and which of its columns hold an edge's keys, its source and its destination.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct EdgeColumns {
    /// Where the edges are read.
    pub(crate) rows: EdgeRows,
    /// The key columns, the file's left out: an edge is walked within one value of them.
    pub(crate) keys: Vec<MadeColumn>,
    /// The source column.
    pub(crate) source: MadeColumn,
    /// The destination column.
    pub(crate) destination: MadeColumn,
}

/// Where a closure reads its edges.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum EdgeRows {
    /// A definition's rows: the edges are a step.
    Definition(DefinitionId),
    /// A source's own rows, read in place by the walk: one row of the source, in cells nothing else names.
    Read {
        /// The source.
        rows: AtomRows,
        /// The row's cells, in the source's column order.
        cells: Vec<FlatTerm>,
        /// Whether the source holds one row a file.
        per_file: RowsPerFile,
    },
}

/// The domain of a nearest walk: what holds the elements a walk may stop at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum NearestDomain {
    /// A definition holds the element in a column.
    Held {
        /// The definition.
        definition: DefinitionId,
        /// Its column that holds the element.
        column: MadeColumn,
    },
    /// A relation's own rows, read in place by the element with the tests of the domain's filter: atoms over cells
    /// nothing else names, `entity` the cell that holds the element.
    Read {
        /// The read of one row, then the filter's tests.
        atoms: Vec<FlatAtom>,
        /// The cell of the element.
        entity: FlatTerm,
    },
}

/// What a nearest walk climbs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum NearestOver {
    /// The tree of a node relation.
    Tree(Relation),
    /// Edges that are a step's rows or a relation's own: asked by the child (the source), a row gives the parent.
    Derived(EdgeColumns),
}
