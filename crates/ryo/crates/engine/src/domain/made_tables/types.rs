//! The types of the `made_tables` concept: the rows of a table the engine makes for a file, and the room its fills work in.
//! Crate-private.

use crate::domain::code::types::{
    CodeWord, FillCode, IndexSite, MemoSite, PoolAt, ProgramCode, RelationNumber,
};
use crate::domain::filled::errors::FilledError;
use crate::domain::filled::types::FilledRelations;
use crate::domain::indexes::types::{FileStamp, IndexArena, RowRun, SourceRow};
use crate::domain::made_tables::matching::types::MatchRoom;
use crate::domain::made_tables::pipelines::types::PipelineRoom;
use crate::domain::steps::types::{MadeTableId, StepRegex};
use ryo_language::domain::facts::types::{CellWord, FactSource, FileWords};
use ryo_language::domain::node_table::types::NodeTable;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_operators::domain::patterns::types::PatternMatcher;
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
    /// The orders of one group's rows, ascending: what the meaning of a group's end reads.
    pub(crate) orders: Vec<Integer>,
    /// The room of an ordered match.
    pub(crate) matching: MatchRoom,
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
    /// The rooms pipelines run in, each taken while its pipeline runs and put back after: a pipeline a pattern's
    /// program starts inside another takes its own.
    pub(crate) rooms: Vec<PipelineRoom>,
    /// The gathered inputs of grouping fills, taken and put back the same way.
    pub(crate) inputs: Vec<TableRows>,
    /// A node column as cells, while an index over the node table is built.
    pub(crate) cells: Vec<CellWord>,
    /// The keys asked of the language through a site while its relation was not whole, by site and cells: each with the
    /// stretch of the store's rows that holds its rows.
    pub(crate) asked: hashbrown::HashTable<AskedStretch>,
    /// The cells of the keys asked, one key after the other.
    pub(crate) asked_cells: Vec<CellWord>,
}

/// A key asked of the language through a site: the site, where its cells are among the cells kept, how many, and the
/// stretch of the store's rows that holds the rows it has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AskedStretch {
    /// The site.
    pub(crate) site: IndexSite,
    /// Where the key's cells start among the cells kept.
    pub(crate) cells: Count,
    /// How many cells the key has.
    pub(crate) width: Count,
    /// The rows it has.
    pub(crate) rows: RowRun,
}

/// What asking a relation for one key answered: the key's rows, or that the relation is whole, with the rows the
/// language wrote to make it so.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum KeyAnswer {
    /// The key's rows.
    Rows(KeyRows),
    /// The relation is whole: its index answers.
    Whole(Count),
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
    /// Of its product stage, when it has one: the rows that entered it. Of an ordered match: its left input's rows.
    pub(crate) left: Count,
    /// Of its product stage: the rows of the source it paired them with. Of an ordered match: its right input's rows.
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
    /// This read made the table: its slot holds the work. A table the loop said has no row is made so, with no fill.
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
    /// A made table read while its fill writes it.
    Written(MadeTableId),
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
    /// The bits every handle of the rows carries besides the row's number: a node's file in the high half, nothing of
    /// another source. The cell a program binds for a row is `tag | row`.
    pub(crate) tag: CellWord,
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
    /// The bits every handle carries besides the row's number: a node's file in the high half, nothing of another
    /// source. The cell a program binds for a row is `tag | row`.
    pub(crate) tag: CellWord,
    /// How many rows the language wrote for this read: nought when they were there.
    pub(crate) filled: Count,
}

/// What the fill side reads of a rule set: its code, the word of each kind's and each field's name by its code, and the
/// sources of its regular expressions by number. The loop builds one from the programs once per scan; the fill side names
/// no type of the programs.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RuleSetView<'a> {
    /// The code.
    pub(crate) code: &'a ProgramCode,
    /// The word of each kind's name, by the kind's code.
    pub(crate) kinds: &'a [CellWord],
    /// The word of each field's name, by the field's code.
    pub(crate) fields: &'a [CellWord],
    /// The sources of the regular expressions, by number.
    pub(crate) regexes: &'a [StepRegex],
}

/// What every read of the fill side is made with: the programs, the file's nodes and text, the language, the file's
/// words, and the store. The loop builds one from its scan request and its scratch; the fill side hands it back to the
/// loop in `PatternRows::pattern_rows`.
pub(crate) struct FillFrame<'a> {
    /// What the fill side reads of the rule set.
    pub(crate) programs: RuleSetView<'a>,
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
    /// What a test of a text is asked of: the operators crate's matcher.
    pub(crate) matcher: PatternMatcher<'a>,
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

/// What the fill side asks of the loop, which implements it: the rows of a definition's pattern for one key (the callback
/// of the `Use` stage), and whether a table it is about to make may have a row. A fill asks a pattern once per distinct key
/// of a batch and runs no step program itself; the loop's memo makes the pattern's program run once per distinct key of
/// the file.
pub(crate) trait PatternRows {
    /// Whether `table` may have a row in this file: the table's own need, which the loop decides and keeps. The fill side
    /// asks before it makes a table it reaches itself (a source of a fill, the table of a site `matched` reads, a table
    /// `scanned` reads); on `Empty` it runs no fill and keeps the table as made with no row, so it asks once per file.
    ///
    /// # Errors
    ///
    /// A source the decision read could not be read.
    fn table_need(
        &mut self,
        frame: &mut FillFrame<'_>,
        table: MadeTableId,
    ) -> Result<TableNeed, FilledError>;

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

    /// Says that the fill of `table` starts: everything the fill side does from here to `table_ended` of the same table
    /// is that table's making, but for what lies between the two calls of a table made inside it (a source its fill
    /// reaches). Said for every table whose fill runs, twice a table and never for a row; a table kept as made with no
    /// row on the loop's `Empty` runs no fill and is not said.
    fn table_started(&mut self, table: MadeTableId);

    /// Says that the fill of `table` ended, with its rows or with a fault.
    fn table_ended(&mut self, table: MadeTableId);
}

/// A part of the code the fill side is asked whether it runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FillPart {
    /// A stage of a pipeline, by its word.
    Stage(CodeWord),
    /// A made table's fill, by its code.
    Fill(FillCode),
}

/// Whether a table may have a row in a file, as the loop decides its need.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TableNeed {
    /// It may: its fill runs.
    Rows,
    /// It has none: no fill runs, and the table is kept as made with no row.
    Empty,
}

/// Whether a pattern has a row for a key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PatternFound {
    /// It has at least one row.
    Rows,
    /// It has none.
    Nothing,
}

/// What having a source there made the engine do: nothing when it was there.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct SourceWork {
    /// How many rows the language wrote.
    pub(crate) filled: Count,
    /// How many rows were indexed.
    pub(crate) indexed: Count,
}

/// Where a fill's own work and what it reached directly are kept while it runs.
#[derive(Debug)]
pub(crate) struct FillKept<'r> {
    /// The fill's own work.
    pub(crate) work: &'r mut FillWork,
    /// What it reached directly.
    pub(crate) reached: &'r mut Vec<Reached>,
}

/// What one run of a pipeline writes to and counts in: the table whose fill runs, where the write goes (the table, or a
/// grouping fill's gathered input), the fill's work, and the room.
#[derive(Debug)]
pub(crate) struct PipeRun<'r> {
    /// The table whose fill runs.
    pub(crate) table: MadeTableId,
    /// Where the pipeline's write goes.
    pub(crate) into: &'r mut TableRows,
    /// The fill's work.
    pub(crate) work: &'r mut FillWork,
    /// The room the pipeline runs in.
    pub(crate) room: &'r mut PipelineRoom,
}

/// What a pipeline's scan reads: its source; of a bucket the site and the key's stretch of that site's index; how many
/// rows; the first row of a whole source; and the bits every handle carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ScanRead {
    /// The source.
    pub(crate) source: ReadSource,
    /// Of a bucket: the site whose index holds it.
    pub(crate) site: Option<IndexSite>,
    /// Of a bucket: the key's stretch of the index's rows. Of a whole source: its rows.
    pub(crate) rows: RowRun,
    /// How many rows the scan reads.
    pub(crate) count: Count,
    /// The bits every handle carries besides the row's number.
    pub(crate) tag: CellWord,
}
