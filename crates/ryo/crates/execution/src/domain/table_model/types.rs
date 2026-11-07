//! The types of the `table_model` concept: plain data, public fields and derives, with no
//! constructor and no inherent impl.

use ryo_relations::domain::relations::types::{CellKind, Value};
use std::sync::Arc;
use ryo_values::domain::rows::types::Table;

use crate::domain::stored_selection::types::{BorrowedPositions, SelectedPositions, SelectionIdentity};
use crate::domain::stored_selection::types::Selector;
use crate::domain::table_cells::types::TreeRows;
use ryo_relations::domain::relations::types::ColumnsView;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::FlagState;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plan_parts::types::LiteralText;
use ryo_values::domain::plan_parts::types::WholeNumber;

/// How many rows a table holds.

/// Which cells of a row say which rows are alike.

/// The rows of a table found by what their key columns hold: each row is chained to the next row of
/// its bucket, in the order the rows came, so a key is found by hashing it and rows with one key are
/// met in the order they came. `sorted` lists the rows by key for the operators that group. The room is
/// reused from one indexing to the next.

/// A row, with the hash of its key.

/// The next row of a chain of an index, plus one; zero where there is none.

/// Rows under typed columns, flat: the cells of every row one after another, each row as wide as
/// there are kinds. A table is cleared and filled again for the next file, keeping its room.

/// The rows an operator reads: the columns of a relation as the store keeps them, or the table
/// another step computed.
#[derive(Clone, Debug)]
pub(crate) enum Rows<'a> {
    /// A relation's columns, read where the store keeps them.
    Stored(ColumnsView<'a>),
    /// Ordered physical row positions and projected columns of a resident relation.
    Selected(SelectedRows<'a>),
    /// The edges of a file's syntax tree, read from the tree itself: no row of them is stored.
    Tree(TreeRows<'a>),
    /// Native tuple positions, decoded only when a consumer asks for a cell.
    Positioned(PositionedRows<'a>),
    /// The rows of an earlier step.
    Computed(&'a Table),
    /// Admitted immutable cross-file rows read without whole-population cell allocation.
    Compact(&'a ryo_values::domain::table_words::types::CompactTable),
}

/// Geometry and positions are borrowed from the same generation-owned publication.
#[derive(Clone, Copy, Debug)]
pub(crate) struct PositionedRows<'a> {
    pub tree: TreeRows<'a>,
    pub positions: &'a crate::domain::positions::types::Positions,
    pub shape: &'a crate::domain::lowering::types::LoweredShape,
    pub kinds: &'a [CellKind],
}
/// A compact row view borrows only its generation's provider; its metadata has owned lifetime.
#[derive(Clone, Debug)]
pub(crate) struct SelectedRows<'a> {
    /// Immutable rectangular provider population.
    pub source: ColumnsView<'a>,
    /// Original row positions and output column map, shared without copying cells.
    pub selection: std::sync::Arc<StoredSelection>,
}

/// Borrowed compact bulk layout without any retained selection capability.
#[derive(Clone, Copy)]
pub(crate) struct SelectedView<'a> {
    pub source: ColumnsView<'a>,
    pub positions: PhysicalRows<'a>,
    pub columns: &'a [ColumnIndex],
}

/// The original rows of a source that a selection keeps, in output order.
#[derive(Clone, Copy)]
pub(crate) struct PhysicalRows<'a>(pub &'a [u32]);

/// Owned, generation-local metadata; it contains no provider/source references.
#[derive(Debug)]
pub(crate) struct StoredSelection {
    /// Provider to reacquire before replay, preserving availability/error checks.
    pub relation: ryo_relations::domain::relations::types::Relation,
    /// Original row IDs in source order, including repeated positions when present.
    pub rows: SelectedPositions,
    /// Physical source columns, in output order and with repeats retained.
    pub columns: Vec<ColumnIndex>,
    /// Types of the projected output columns, including zero-column projections.
    pub kinds: Vec<CellKind>,
    /// Exact ordered physical predicates and referenced bindings that selected these positions.
    pub identity: std::sync::Arc<SelectionIdentity>,
}

/// The rows of one input that the per-file steps hand to the cross-file steps: the rows of every
/// file that has any, one after another.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Frontier {
    /// What the rows are of.
    pub input: Input,
    /// The rows, with every name as the place of its text.
    pub table: std::sync::Arc<Table>,
}

/// The number a value is for hashing: what it is stored as, apart from which kind of value it is.

/// The hash of the cells that make a key.

/// Whether a value, a kind or an entity, is in a set of them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SetMembership {
    /// It is in the set.
    In,
    /// It is not.
    Out,
}

/// Whether a row was put into a table by the call that looked for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Addition {
    /// It was not there before, and now is.
    Added,
    /// It was there already.
    Existing,
}

/// What a comparison decided about a text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Verdict {
    /// The comparison holds.
    Holds,
    /// It does not.
    Fails,
}

use ryo_values::domain::rows::types::RowIndex;

/// Opaque immutable publication crossing the engine's domain port. A foreign
/// implementation carries no authority: the application verifies its private
/// record and current population epoch before admission or resolution.
pub(crate) trait PublishedRelation: std::any::Any + std::fmt::Debug + Send + Sync {
    fn as_any(&self) -> &dyn std::any::Any;
}
/// Reuse the existing selector compiler; neither member owns execution state.
#[derive(Clone, Debug, Default)]
pub(crate) struct PreparedPhysical {
    pub selector: Option<Arc<Selector>>,
}

pub(crate) mod registered_representation;

/// The representation a physical row source is held in.
#[derive(Clone, Copy)]
pub(crate) enum RepresentationInput<'a> {
    Stored(ColumnsView<'a>),
    Selected {
        source: ColumnsView<'a>,
        positions: BorrowedPositions<'a>,
        columns: &'a [ColumnIndex],
        kinds: &'a [CellKind],
    },
    Tree(TreeRows<'a>),
    Positioned(PositionedRows<'a>),
    Computed(&'a Table),
    Compact(&'a ryo_values::domain::table_words::types::CompactTable),
}

impl<'f> From<&'f Rows<'f>> for RepresentationInput<'f> {
    fn from(rows: &'f Rows<'f>) -> Self {
        match rows {
            Rows::Stored(source) => Self::Stored(*source),
            Rows::Selected(view) => Self::Selected {
                source: view.source,
                positions: BorrowedPositions(&view.selection.rows),
                columns: &view.selection.columns,
                kinds: &view.selection.kinds,
            },
            Rows::Tree(tree) => Self::Tree(*tree),
            Rows::Computed(table) => Self::Computed(table),
            Rows::Compact(table) => Self::Compact(table),
            Rows::Positioned(view) => Self::Positioned(*view),
        }
    }
}
