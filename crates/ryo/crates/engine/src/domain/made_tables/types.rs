//! The types of the `made_tables` concept: the rows of a table the engine makes for a file, and the room its fills work in.
//! Crate-private.

use crate::domain::code::types::{IndexSite, MemoSite, PoolAt, RelationNumber};
use crate::domain::filled::errors::FilledError;
use crate::domain::filled::types::FilledRelations;
use crate::domain::indexes::types::{FileStamp, IndexArena, RowRun, SourceRow};
use crate::domain::programs::types::ProgramsEngine;
use crate::domain::steps::types::MadeTableId;
use ryo_language::domain::facts::types::{CellWord, FactSource, FileWords};
use ryo_language::domain::node_table::types::NodeTable;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_relations::domain::relations::types::{Count, FileId, Integer, NodeExtent};
use ryo_values::domain::plan_parts::types::ColumnIndex;

/// The rows of one table for a file: its cells row by row, kept between files and emptied, never freed.
#[derive(Debug, Default)]
pub(crate) struct TableRows {
    /// The cells, row by row.
    pub(crate) cells: Vec<CellWord>,
    /// How many cells a row has.
    pub(crate) width: Count,
    /// How many rows it holds: a table of no column has rows too.
    pub(crate) count: Count,
    /// The rows it holds, by what they hold: what a write that drops repeats asks.
    pub(crate) seen: hashbrown::HashTable<SourceRow>,
    /// Of a sorted table: its order column, row by row, as the numbers a `Latest` searches.
    pub(crate) orders: Vec<Integer>,
}

/// The room a fill that groups its input works in: the input's rows as gathered, the index that groups them, and one row
/// being written. Kept between fills, never freed.
#[derive(Debug, Default)]
pub(crate) struct GroupRoom {
    /// The index of the input by its keys.
    pub(crate) index: IndexArena,
    /// The key columns as the operators crate names them.
    pub(crate) keys: Vec<ColumnIndex>,
    /// One group's row, as its meaning wrote it.
    pub(crate) row: Vec<CellWord>,
    /// The values of the counted column in one group, to count the distinct ones.
    pub(crate) values: Vec<CellWord>,
}

/// Everything of the fill side a worker keeps for a file: the filled relations, the indexes of every source, and the made
/// tables. Emptied per file by `sources::service::next_file`, never freed. The loop's scratch embeds one and reads it only
/// through `made_tables::sources::service`.
#[derive(Debug, Default)]
pub(crate) struct FillStore {
    /// The file's number in the worker's life.
    pub(crate) stamp: FileStamp,
    /// The relations, by the number the code gives each.
    pub(crate) relations: FilledRelations,
    /// The indexes of every source, by `IndexSite`; and the rows a key asked of the language holds.
    pub(crate) indexes: IndexArena,
    /// The made tables, by number.
    pub(crate) tables: Vec<TableSlot>,
    /// The room a grouping fill works in.
    pub(crate) room: GroupRoom,
}

/// Whether a table is made for the file.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum MadeState {
    /// No read of the file asked for it.
    #[default]
    Unmade,
    /// Its fill is running.
    Making,
    /// It is made: it is never made again for the file.
    Made,
}

/// One made table in a file: its rows, what making it cost, and what its fill reached directly.
#[derive(Debug, Default)]
pub(crate) struct TableSlot {
    /// The file it was made for.
    pub(crate) stamp: FileStamp,
    /// Whether it is made.
    pub(crate) state: MadeState,
    /// Its rows.
    pub(crate) rows: TableRows,
    /// What its own fill cost, in rows: nothing of what it reached.
    pub(crate) work: FillWork,
    /// What its fill read directly: other tables, relations, indexes, the node table. A repeat is harmless.
    pub(crate) reached: Vec<Reached>,
}

/// What one table's own fill cost, in rows, each added once per batch.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct FillWork {
    /// The rows its stages read.
    pub(crate) read: Count,
    /// The rows its stages gave.
    pub(crate) given: Count,
    /// The rows the table holds.
    pub(crate) made: Count,
    /// Of its product stage, when it has one: the rows that entered it.
    pub(crate) left: Count,
    /// Of its product stage: the rows of the source it paired them with.
    pub(crate) right: Count,
}

/// What a fill read directly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Reached {
    /// Another made table.
    Table(MadeTableId),
    /// A relation.
    Relation(RelationNumber),
    /// An index.
    Index(IndexSite),
    /// The node table.
    Nodes,
}

/// Whether a read made its table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MadeNow {
    /// This read made the table: its slot holds the work.
    Made,
    /// An earlier read of the file made it.
    Held,
}

/// The rows of a source a program or a pipeline reads, as its word names them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReadSource {
    /// The node table: the file's own tree alone, or the embedded segments too.
    Nodes(NodeExtent),
    /// A filled relation.
    Filled(RelationNumber),
    /// A made table.
    Made(MadeTableId),
}

/// Where a key's cells come from: what decides how a relation that is not whole is asked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum KeyFrom {
    /// A register, or a column of a batch, among them: the language is asked for that key alone.
    Row,
    /// Constants alone: the relation is filled whole and indexed.
    Constants,
}

/// The rows a key holds, and what finding them made the engine do.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct KeyRows {
    /// The rows, a stretch of the store's rows: read with `sources::service::rows_at`.
    pub(crate) rows: RowRun,
    /// How many rows the language wrote for this read: nought when they were there.
    pub(crate) filled: Count,
    /// How many rows this read indexed: nought when the index was built.
    pub(crate) indexed: Count,
}

/// Every row of a source, and what reading it made the engine do.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct WholeRows {
    /// The rows' handles, from the first to past the last: a relation filled whole after keys starts past their rows.
    pub(crate) rows: RowRun,
    /// How many rows the language wrote for this read: nought when they were there.
    pub(crate) filled: Count,
}

/// What every read of the fill side is made with: the programs, the file's nodes and text, the language, the file's
/// words, and the store. The loop builds one from its scan request and its scratch; the fill side hands it back to the
/// loop in `PatternRows::pattern_rows`.
pub(crate) struct FillFrame<'a> {
    /// The rule set's programs.
    pub(crate) programs: &'a ProgramsEngine,
    /// The file's nodes.
    pub(crate) nodes: &'a NodeTable,
    /// The file.
    pub(crate) file: FileId,
    /// The file's bytes.
    pub(crate) text: ByteSlice<'a>,
    /// The language.
    pub(crate) facts: &'a mut dyn FactSource,
    /// The file's words.
    pub(crate) words: &'a mut FileWords,
    /// The fill side's state for the file.
    pub(crate) store: &'a mut FillStore,
}

impl std::fmt::Debug for FillFrame<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

/// One ask of a `Use` stage: the pattern in call form by its body record and memo site, the cells of its bound formals
/// in the record's order, and the table whose fill asks.
#[derive(Clone, Copy, Debug)]
pub(crate) struct PatternAsk<'k> {
    /// The table whose fill asks.
    pub(crate) asker: MadeTableId,
    /// The body record of the pattern, in call form.
    pub(crate) body: PoolAt,
    /// The pattern's memo site.
    pub(crate) memo: MemoSite,
    /// The cell of each bound formal, in the record's order.
    pub(crate) key: &'k [CellWord],
}

/// The callback of the `Use` stage, which the loop implements: the rows of a definition's pattern for one key. A fill asks
/// once per distinct key of a batch and runs no step program itself; the loop's memo makes the pattern's program run once
/// per distinct key of the file.
pub(crate) trait PatternRows {
    /// Appends to `rows` the rows the pattern yields for the key, row by row, each the cells of its collected columns
    /// ascending; a pattern that collects nothing appends nothing and answers whether it has a row.
    ///
    /// # Errors
    ///
    /// A source the pattern's program read could not be read.
    fn pattern_rows(
        &mut self,
        frame: &mut FillFrame<'_>,
        asked: PatternAsk<'_>,
        rows: &mut Vec<CellWord>,
    ) -> Result<PatternFound, FilledError>;
}

/// Whether a pattern has a row for a key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PatternFound {
    /// It has at least one row.
    Rows,
    /// It has none.
    Nothing,
}
