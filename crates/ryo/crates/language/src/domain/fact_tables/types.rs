//! The types of the `fact_tables` concept: the one request the engine makes of a language for a file (`FactRequest`: the
//! relations the file's surviving rules read), the set of tables the language answers it with (`FactSet`, each table written
//! through a `FactWriter`), and the keys and rows the engine then reads it by, like a small database of the file. Each
//! table's layout, its indexes and the source that wrote it are private: the engine reads every table through the one lookup
//! of `fact_tables::service` and never asks the language again for the file.

use super::keying::types::{KeyBits, KeyedRow};
use crate::domain::facts::types::{CellWord, EntityId, FactAnswer, PartialRows};
use ryo_relations::domain::relations::types::{Column, Count, Position};

/// The one request of a file: the relations its surviving rules read, one bit a relation by its place in the catalog's
/// list. Made by `fact_tables::service::request_of`.
///
/// ```compile_fail,E0451
/// let forged = ryo_language::domain::fact_tables::types::FactRequest { relations: Vec::new() };
/// ```
///
/// # Examples
///
/// ```
/// use ryo_language::domain::fact_tables::service::request_of;
/// use ryo_language::domain::fact_tables::types::FactRequest;
/// use ryo_relations::domain::relations::types::Relation;
///
/// assert_ne!(request_of(&[Relation::Comment]), FactRequest::default());
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct FactRequest {
    /// The relations, sixty-four a word.
    pub(crate) relations: Vec<RelationBits>,
}

/// Sixty-four relations of a request, one bit each, by their places in the catalog's list.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub(crate) struct RelationBits(pub(crate) u64);

/// A key a table's rows are found by: a node of the file, a word of the file, or an entity of the run (a module, a project
/// symbol), whose id is the same in every file.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::fact_tables::types::FactKey;
/// use ryo_relations::domain::relations::types::Position;
///
/// assert_ne!(FactKey::Node(Position(1)), FactKey::Node(Position(2)));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FactKey {
    /// A node of the file, by its place in the node table.
    Node(Position),
    /// A word of the file or of the run.
    Word(CellWord),
    /// An entity of the run.
    Entity(EntityId),
    /// A cell as the key column holds it: a node's id, a word's code or an entity's id, read by the column's own space.
    Cell(CellWord),
}

/// One row of a table, by its number: what `rows_of` and `every_row` hand out, and the place in `cells_of`'s cells.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::fact_tables::types::FactRow;
///
/// assert!(FactRow(0) < FactRow(1));
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FactRow(pub u32);

/// The rows `first` up to, not including, `end`.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::fact_tables::types::{FactRow, FactRowRun};
///
/// let run = FactRowRun { first: FactRow(2), end: FactRow(5) };
/// assert_eq!(run.end.0 - run.first.0, 3);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct FactRowRun {
    /// The first row.
    pub first: FactRow,
    /// The row after the last.
    pub end: FactRow,
}

/// The rows of one key, in the order they were written: a run of rows when the table is in that key's order, or a list of
/// rows when it is in another key's order.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::fact_tables::types::{FactRow, FactRowRun, FactRowList};
///
/// let rows: Vec<FactRow> = FactRowList::Run(FactRowRun { first: FactRow(1), end: FactRow(3) }).into_iter().collect();
/// assert_eq!(rows, [FactRow(1), FactRow(2)]);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FactRowList<'t> {
    /// These rows, one after another.
    Run(FactRowRun),
    /// These rows.
    Listed(&'t [FactRow]),
}

/// The rows of a `FactRowList`, one at a time: made by its `into_iter`.
///
/// ```compile_fail,E0451
/// fn forged(walk: ryo_language::domain::fact_tables::types::FactRowListIter<'_>) {
///     let _again = ryo_language::domain::fact_tables::types::FactRowListIter { walk: walk.walk };
/// }
/// ```
///
/// # Examples
///
/// ```
/// use ryo_language::domain::fact_tables::types::{FactRow, FactRowList};
///
/// let rows = [FactRow(4)];
/// assert_eq!(FactRowList::Listed(&rows).into_iter().next(), Some(FactRow(4)));
/// ```
#[derive(Clone, Debug)]
pub struct FactRowListIter<'t> {
    /// Where the walk is.
    pub(crate) walk: RowsWalk<'t>,
}

/// The private walk of a `FactRowListIter`.
#[derive(Clone, Debug)]
pub(crate) enum RowsWalk<'t> {
    /// Through a run: the next row and the end.
    Run(FactRowRun),
    /// Through a list.
    Listed(std::slice::Iter<'t, FactRow>),
}

impl<'t> IntoIterator for FactRowList<'t> {
    type Item = FactRow;
    type IntoIter = FactRowListIter<'t>;

    fn into_iter(self) -> Self::IntoIter {
        FactRowListIter {
            walk: match self {
                FactRowList::Run(run) => RowsWalk::Run(run),
                FactRowList::Listed(rows) => RowsWalk::Listed(rows.iter()),
            },
        }
    }
}

impl Iterator for FactRowListIter<'_> {
    type Item = FactRow;

    fn next(&mut self) -> Option<FactRow> {
        match &mut self.walk {
            RowsWalk::Run(run) => (run.first < run.end).then(|| {
                let next = run.first;
                run.first = FactRow(next.0.saturating_add(1));
                next
            }),
            RowsWalk::Listed(rows) => rows.next().copied(),
        }
    }
}

/// Whether a key has a row in a table.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::fact_tables::types::KeyPresence;
///
/// assert_ne!(KeyPresence::Held, KeyPresence::Absent);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KeyPresence {
    /// The key has at least one row.
    Held,
    /// The key has none.
    Absent,
}

/// The cells of one row of a relation, in the relation's column order (what a language hands the writer for each row), or
/// the cells of one column, one a row (what `cells_of` answers).
///
/// # Examples
///
/// ```
/// use ryo_language::domain::fact_tables::types::FactCells;
/// use ryo_language::domain::facts::types::CellWord;
///
/// let cells = [CellWord(3), CellWord(4)];
/// assert_eq!(FactCells(&cells).0.len(), 2);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FactCells<'c>(pub &'c [CellWord]);

/// One file's facts as a language answered its request: a table and an answer for each relation asked. The engine reads only
/// this after the request; its buffers are kept from file to file.
///
/// ```compile_fail,E0451
/// let forged = ryo_language::domain::fact_tables::types::FactSet { body: Default::default() };
/// ```
///
/// # Examples
///
/// ```
/// use ryo_language::domain::fact_tables::service::{answer_of, begun};
/// use ryo_language::domain::fact_tables::types::{FactRequest, FactSet};
/// use ryo_language::domain::facts::types::FactAnswer;
/// use ryo_relations::domain::answers::types::AbsenceCause;
/// use ryo_relations::domain::relations::types::Relation;
///
/// let mut set = FactSet::default();
/// begun(&mut set, &FactRequest::default());
/// assert_eq!(answer_of(&set, Relation::Comment), FactAnswer::Refused(AbsenceCause::NotYetRead));
/// ```
#[derive(Clone, Debug, Default)]
pub struct FactSet {
    /// The private body.
    pub(crate) body: SetBody,
}

/// The private body of a `FactSet`.
#[derive(Clone, Debug, Default)]
pub(crate) struct SetBody {
    /// The request answered.
    pub(crate) request: FactRequest,
    /// Each relation's table, by its place in the catalog's list.
    pub(crate) tables: Vec<FactTable>,
    /// Each relation's answer, by the same place.
    pub(crate) answers: Vec<FactAnswer>,
}

/// One relation of one file as a language wrote it, found by each key column the catalog declares for the relation. Its
/// layout, its indexes and its columns are private.
///
/// ```compile_fail,E0451
/// let forged = ryo_language::domain::fact_tables::types::FactTable { body: Default::default() };
/// ```
///
/// # Examples
///
/// ```
/// use ryo_language::domain::fact_tables::reading::service::every_row;
/// use ryo_language::domain::fact_tables::types::{FactRowRun, FactTable};
///
/// assert_eq!(every_row(&FactTable::default()), FactRowRun::default());
/// ```
#[derive(Clone, Debug, Default)]
pub struct FactTable {
    /// The private body.
    pub(crate) body: TableBody,
}

/// The private body of a `FactTable`.
#[derive(Clone, Debug, Default)]
pub(crate) struct TableBody {
    /// The relation's columns by place, as the catalog names them.
    pub(crate) named: Vec<Column>,
    /// The table's indexes: the first is the key the rows are in the order of, when the relation has one; then the others.
    pub(crate) indexes: Vec<KeyColumnIndex>,
    /// The columns by place: one cell a row.
    pub(crate) columns: Vec<Vec<CellWord>>,
    /// How many rows.
    pub(crate) rows: Count,
    /// Room the rows are put in key order with.
    pub(crate) sorting: SortRoom,
}

/// One key column's index over a table's rows.
#[derive(Clone, Debug)]
pub(crate) struct KeyColumnIndex {
    /// The column's place among the relation's columns.
    pub(crate) place: Count,
    /// The column.
    pub(crate) column: Column,
    /// What the column's cells number.
    pub(crate) space: KeySpace,
    /// Whether the rows are in this key's order.
    pub(crate) order: IndexOrder,
    /// How a key's rows are found.
    pub(crate) layout: TableLayout,
    /// One bit a key number: the key has a row.
    pub(crate) present: Vec<KeyBits>,
    /// Per word of `present`, how many keys before it have rows.
    pub(crate) rank: Vec<KeyRank>,
    /// `TableLayout::Listed`: where each present key's rows start among the rows in key order; one more entry at the end.
    pub(crate) offsets: Vec<FactRow>,
    /// `IndexOrder::Other`: the rows in this key's order.
    pub(crate) listed: Vec<FactRow>,
    /// The key number of each row, while the table is written.
    pub(crate) keys: Vec<KeyCode>,
}

/// Whether a table's rows are in an index's key order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum IndexOrder {
    /// They are: a key's rows are a run.
    #[default]
    Keyed,
    /// They are in another key's order: a key's rows are a stretch of `KeyColumnIndex::listed`.
    Other,
}

/// How many keys before a word of `KeyBits` have rows: the rank a key's row or rows are found by.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct KeyRank(pub(crate) u32);

/// A key as a table numbers it: a node's place, a word's code or an entity's id.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct KeyCode(pub(crate) u32);

/// The room a table puts its rows in key order with, kept from file to file.
#[derive(Clone, Debug, Default)]
pub(crate) struct SortRoom {
    /// Each row's key and number, one word a row.
    pub(crate) pairs: Vec<KeyedRow>,
    /// One column's cells in their new order.
    pub(crate) cells: Vec<CellWord>,
    /// One index's keys in their new order.
    pub(crate) keys: Vec<KeyCode>,
}

/// What a table's key numbers are.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum KeySpace {
    /// Nodes of the file, by place.
    #[default]
    Nodes,
    /// Words of the file or of the run.
    Words,
    /// Entities of the run.
    Entities,
}

/// How an index's rows are found by key.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum TableLayout {
    /// One row at most a key: a key's row is its rank among the present keys: a per-node column.
    #[default]
    PerKey,
    /// Any number of rows a key: a key's rows are a stretch of `offsets`: a keyed list.
    Listed,
}

/// Where a language writes one relation of its answer: a row at a time, then `finished`. Made by
/// `fact_tables::service::writer`, which empties the relation's table.
///
/// ```compile_fail,E0451
/// fn forged(set: &mut ryo_language::domain::fact_tables::types::FactSet) {
///     let _writer = ryo_language::domain::fact_tables::types::FactWriter { set, place: Default::default() };
/// }
/// ```
///
/// # Examples
///
/// ```
/// use ryo_language::domain::fact_tables::service::{answer_of, begun, finished, request_of, writer};
/// use ryo_language::domain::fact_tables::types::FactSet;
/// use ryo_language::domain::facts::types::FactAnswer;
/// use ryo_relations::domain::relations::types::Relation;
///
/// let mut set = FactSet::default();
/// begun(&mut set, &request_of(&[Relation::Comment]));
/// finished(writer(&mut set, Relation::Comment));
/// assert_eq!(answer_of(&set, Relation::Comment), FactAnswer::Answered);
/// ```
#[derive(Debug)]
pub struct FactWriter<'s> {
    /// The set written.
    pub(crate) set: &'s mut FactSet,
    /// The relation's place in the catalog's list.
    pub(crate) place: Count,
    /// What the language said of the rows it could not write.
    pub(crate) partial: PartialSaid,
}

/// What a language said of the rows it could not write.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) enum PartialSaid {
    /// Nothing: every row is there.
    #[default]
    Whole,
    /// Some rows may be missing.
    Partial(PartialRows),
}
