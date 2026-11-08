//! The types of the `table_model` concept: plain data, public fields and derives, with no
//! constructor and no inherent impl.

use ryo_values::domain::rows::types::Table;

use crate::domain::table_cells::types::TreeRows;
use ryo_relations::domain::relations::types::ColumnsView;
use ryo_values::domain::plan_parts::types::Input;

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
pub enum Rows<'a> {
    /// A relation's columns, read where the store keeps them.
    Stored(ColumnsView<'a>),
    /// The edges of a file's syntax tree, read from the tree itself: no row of them is stored.
    Tree(TreeRows<'a>),
    /// The rows of an earlier step.
    Computed(&'a Table),
    /// Admitted immutable cross-file rows read without whole-population cell allocation.
    Compact(&'a ryo_values::domain::table_words::types::CompactTable),
}

/// The rows of one input that the per-file steps hand to the cross-file steps: the rows of every
/// file that has any, one after another.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frontier {
    /// What the rows are of.
    pub input: Input,
    /// The rows, with every name as the place of its text.
    pub table: std::sync::Arc<Table>,
}

/// The number a value is for hashing: what it is stored as, apart from which kind of value it is.

/// The hash of the cells that make a key.

/// Whether a value, a kind or an entity, is in a set of them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetMembership {
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
pub enum Verdict {
    /// The comparison holds.
    Holds,
    /// It does not.
    Fails,
}


/// Opaque immutable publication crossing the engine's domain port. A foreign
/// implementation carries no authority: the application verifies its private
/// record and current population epoch before admission or resolution.
pub trait PublishedRelation: std::any::Any + std::fmt::Debug + Send + Sync {
    fn as_any(&self) -> &dyn std::any::Any;
}
pub mod registered_representation;

/// The representation a physical row source is held in.
#[derive(Clone, Copy)]
pub enum RepresentationInput<'a> {
    Stored(ColumnsView<'a>),
    Tree(TreeRows<'a>),
    Computed(&'a Table),
    Compact(&'a ryo_values::domain::table_words::types::CompactTable),
}

impl<'f> From<&'f Rows<'f>> for RepresentationInput<'f> {
    fn from(rows: &'f Rows<'f>) -> Self {
        match rows {
            Rows::Stored(source) => Self::Stored(*source),
            Rows::Tree(tree) => Self::Tree(*tree),
            Rows::Computed(table) => Self::Computed(table),
            Rows::Compact(table) => Self::Compact(table),
        }
    }
}
