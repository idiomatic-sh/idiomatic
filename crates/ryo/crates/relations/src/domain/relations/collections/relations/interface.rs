//! The shape every relation of the catalog implements: its row, its columns and the numbers a row is stored as.

use crate::domain::catalog::types::{RelationNotes, RowScope};
use crate::domain::relations::types::{Bits, CellKind, Class, Column, Relation};
use crate::domain::relations::types::{FactForm, NodeColumn, NodeExtent, RelationSize};

/// Seals [`RelationsItem`]: only the catalog's own relations implement it, because a language defines no shape of its own.
#[expect(
    clippy::inline_modules,
    reason = "a sealed trait needs a public trait in a module the crate keeps private: a module of one empty trait, which as a file would be a file of no role the collection places"
)]
pub(crate) mod sealed {
    /// Implemented by every relation of the catalog, and by nothing else.
    pub(crate) trait Sealed {}
}

/// What every relation offers. A relation is named by its type, never by text. Sealed: the catalog is the only place a relation is defined.
pub trait RelationsItem: sealed::Sealed + 'static {
    /// One row of the relation.
    type Row;
    /// The relation's columns.
    type Column: Copy;
    /// A row as numbers, one for each column, in column order.
    type Words: AsRef<[Bits]>;

    /// What the relation is about, which the checks of a language's answers are made by: a language that says it is not code reads no
    /// relation of code. A relation added to the catalog says its class, or it does not compile.
    const CLASS: Class;

    /// Which files the relation's rows are about, which the partition of a step that reads it follows. A relation added to the
    /// catalog says its scope, or it does not compile.
    const SCOPE: RowScope;

    /// Whether the relation's rows are made from the notes the languages write of the files. A relation added to the catalog says it,
    /// or it does not compile.
    const NOTES: RelationNotes;

    /// How the engine reaches the relation's rows: read in place from the node table, or filled by the language once per file and
    /// indexed by its key. A relation added to the catalog says it, or it does not compile.
    const FORM: FactForm;

    /// For a `NodeTable` relation, the node-table column each of its columns is read from, every column once; empty for a filled
    /// relation.
    const NODE_COLUMNS: &'static [(Self::Column, NodeColumn)];

    /// For a `NodeTable` relation, which ids of the table it has rows for: the file's own tree, or the embedded segments too.
    /// Nothing for a filled relation. A `NodeTable` relation added to the catalog says it, or it does not compile.
    const EXTENT: Option<NodeExtent>;

    /// For a filled relation, how its rows grow with the file. Nothing for a `NodeTable` relation, whose rows are the table's
    /// elements. A filled relation added to the catalog says it, or it does not compile.
    const SIZE: Option<RelationSize>;

    /// For a filled relation, the columns every language that fills it promises a fill by key for; none for a `NodeTable`
    /// relation. A filled relation added to the catalog says them (an empty list promises none), or it does not compile.
    const KEYS: &'static [Column];

    /// Which relation this is.
    fn relation() -> Relation;

    /// Every column, in the order the rows hold them.
    fn columns() -> Vec<Self::Column>;

    /// What each column holds, in column order.
    fn kinds() -> &'static [CellKind];

    /// What a column holds.
    fn kind(column: Self::Column) -> CellKind;

    /// A row as the numbers the store keeps, with no allocation.
    fn words(row: Self::Row) -> Self::Words;
}
