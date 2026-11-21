//! The types of the `filled` concept: one relation's rows in a file as the language wrote them, and which of them were
//! asked for. Crate-private.

use crate::domain::code::types::{FillCode, Op, RelationNumber};
use crate::domain::indexes::types::{FileStamp, RowRun};
use ryo_language::domain::facts::types::{
    CellWord, FactSource, FileWords, FilledColumns, PartialRows,
};
use ryo_relations::domain::answers::types::AbsenceCause;
use ryo_relations::domain::relations::types::{Column, Count, Relation};

/// How much of a relation a file holds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum FilledState {
    /// Nothing was asked.
    #[default]
    Unfilled,
    /// Some keys were asked, each alone.
    Keyed,
    /// Every row is there, from `RelationSlot::whole` on.
    Whole,
}

/// A key the language was asked for: the column by its place in the relation, the cell, and the rows the answer appended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AskedKey {
    /// The column's place in the relation.
    pub(crate) column: Count,
    /// The cell.
    pub(crate) cell: CellWord,
    /// The rows the answer appended, by row number.
    pub(crate) rows: RowRun,
}

/// One relation in a file: its columns, their buffers kept between files; the keys asked; where the whole fill starts.
#[derive(Debug, Default)]
pub(crate) struct RelationSlot {
    /// The file the rows are of.
    pub(crate) stamp: FileStamp,
    /// How much of the relation is there.
    pub(crate) state: FilledState,
    /// The columns, each row a handle: its number.
    pub(crate) columns: FilledColumns,
    /// How many rows the columns hold.
    pub(crate) rows: Count,
    /// Where the whole fill's rows start: after the rows the keys asked before it appended.
    pub(crate) whole: Count,
    /// The keys asked, by column and cell.
    pub(crate) asked: hashbrown::HashTable<AskedKey>,
    /// The relation's columns by place, as the catalog names them: written the first time the worker reads the relation.
    pub(crate) named: Vec<Column>,
    /// What the language answered when it did not write the rows: every later read of the file answers the same.
    pub(crate) fault: Option<KeptFault>,
}

/// A worker's relations, by the number the code gives each (`ProgramCode::relations`).
#[derive(Debug, Default)]
pub(crate) struct FilledRelations(pub(crate) Vec<RelationSlot>);

/// What one ask of the language wrote: the rows by number, and whether the relation is whole after it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct AskedRows {
    /// The rows that hold what was asked: of a key its rows, of the whole every row of the whole fill.
    pub(crate) rows: RowRun,
    /// How many rows the language wrote for this ask: nought when the rows were there.
    pub(crate) filled: Count,
    /// How much of the relation is there after the ask.
    pub(crate) state: FilledState,
}

/// What an ask is made with: the relation by its number and its name, the file's stamp, the language and the file's words.
pub(crate) struct RelationAsk<'a> {
    /// The relation's number in the code.
    pub(crate) number: RelationNumber,
    /// The relation.
    pub(crate) relation: Relation,
    /// The file.
    pub(crate) stamp: FileStamp,
    /// The language.
    pub(crate) facts: &'a mut dyn FactSource,
    /// The file's words.
    pub(crate) words: &'a mut FileWords,
}

impl std::fmt::Debug for RelationAsk<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

/// What of the contract the fill side does not run yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UnbuiltFill {
    /// A fill, by its code.
    Fill(FillCode),
    /// A stage, by its word's code.
    Stage(Op),
    /// A read of a table while it is written.
    Written,
    /// An index over the node table that sorts each key's rows.
    OrderedNodes,
}

/// What a word or a record named that the code does not hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MissingCode {
    /// A made table.
    Table,
    /// A relation.
    Relation,
    /// An index site.
    Site,
    /// A place in the pool.
    Pool,
    /// A constant.
    Constant,
    /// A word.
    Word,
    /// A column of a source.
    Column,
}

/// What a language answered when it did not write a relation's rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum KeptFault {
    /// It refuses the relation for the file.
    Refused(AbsenceCause),
    /// Some rows may be missing.
    Partial(PartialRows),
}
