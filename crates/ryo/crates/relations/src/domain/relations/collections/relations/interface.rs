//! The shape every relation of the catalog implements: its row, its columns and the numbers a row is stored as.

use crate::domain::relations::types::{Bits, CellKind, Class, Relation};

/// Seals [`RelationsItem`]: only the catalog's own relations implement it, because a language defines no shape of its own.
#[expect(
    clippy::inline_modules,
    reason = "a sealed trait needs a public trait in a module the crate keeps private: a module of one empty trait, which as a file would be a file of no role the collection places"
)]
pub(crate) mod sealed {
    /// Implemented by every relation of the catalog, and by nothing else.
    #[expect(
        unnameable_types,
        reason = "a sealed trait is unnameable outside the crate on purpose: that is what keeps others from implementing it"
    )]
    pub trait Sealed {}
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
