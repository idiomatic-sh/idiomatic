//! The types of the `flattening` part of the lowering: a plan step as conjunctive branches of atoms over terms. A branch is
//! the unit the planning orders and the two emitters write: steps for a program, stages for a table's pipeline.

use crate::domain::steps::types::{
    AxisDirection, DefinitionId, MadeColumn, MadeTableId, Polarity, TestOperation,
};
use ryo_relations::domain::relations::types::{Integer, Relation};
use ryo_values::domain::plan_orders::types::OrderBound;

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
    /// A definition must have, or must not have, a row holding the given cells.
    Decided {
        /// Which.
        polarity: Polarity,
        /// The definition.
        definition: DefinitionId,
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
        /// The node relation whose tree is walked.
        relation: Relation,
        /// The definition of the domain.
        domain: DefinitionId,
        /// The domain's column that holds the element.
        column: MadeColumn,
        /// The element walked from.
        start: FlatTerm,
        /// The ancestor reached.
        reached: FlatTerm,
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

/// A column of a row, or a cell of an atom, by place.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ColumnPlace(pub(crate) usize);

/// Two terms an equality holds equal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct EqualTerms {
    /// One term.
    pub(crate) left: FlatTerm,
    /// The other.
    pub(crate) right: FlatTerm,
}
