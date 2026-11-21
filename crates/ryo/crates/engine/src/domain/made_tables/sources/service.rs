//! The services of the `sources` part: what a program's step or a pipeline's stage asks of the fill side. Every source is
//! read through here: a made table is made on its first read and once, a relation is asked of the language whole or for
//! one key, an index is built by the columns asked the first time it is read.

use crate::domain::code::small_codes::service::node_column_of;
use crate::domain::code::types::{IndexSite, RelationNumber, Short};
use crate::domain::filled::errors::FilledError;
use crate::domain::filled::service::rows_held;
use crate::domain::filled::types::UnbuiltFill;
use crate::domain::indexes::reading::service::{run_of, stretch_of};
use crate::domain::indexes::service::clear_indexes;
use crate::domain::indexes::types::SourceRow;
use crate::domain::made_tables::keyed::service::key_asked;
use crate::domain::made_tables::making::service::{ensured, made_now};
use crate::domain::made_tables::reading::service::{
    cell_held, column_held, index_at, node_names, rows_in, site_at, site_source, tag_of,
};
use crate::domain::made_tables::types::{
    FillFrame, FillStore, KeyAnswer, KeyFrom, KeyRows, MadeNow, PatternRows, Reached, ReadSource,
    TableSlot, WholeRows,
};
use crate::domain::stages::nodes::service::node_cell;
use crate::domain::stages::service::tagged;
use crate::domain::steps::types::MadeTableId;
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::Count;

/// The cell of a node column whose code names no column: no word and no number of any row.
const NO_CELL: CellWord = CellWord(u64::MAX);

/// Starts a file: every relation, index and table of the file before is unset, its buffers kept.
pub(crate) fn next_file(store: &mut FillStore) {
    store.stamp.0 = store.stamp.0.wrapping_add(1).max(1);
    clear_indexes(&mut store.indexes);
    store.asked.clear();
    store.asked_cells.clear();
}

/// Makes `table` for the file if no read of the file made it: runs its fill once, its pipelines over batches, asking
/// `uses` for the rows of each pattern in call form a `Use` stage names. The table's slot then holds its rows, the work
/// of its own fill and what the fill reached directly. The caller has decided the table's own need: `made` does not ask
/// it. Every other table the fill side reaches on the way (a source of this fill, the table of a site, a table scanned)
/// it asks `uses.table_need` of first, and makes with no row and no fill on `Empty`.
///
/// # Errors
///
/// A source its fill reads could not be read, or the fill is one the fill side does not run yet.
pub(crate) fn made(
    frame: &mut FillFrame<'_>,
    uses: &mut dyn PatternRows,
    table: MadeTableId,
) -> Result<MadeNow, FilledError> {
    made_now(frame, uses, table)
}

/// A table's slot: its rows, its work, what it reached; nothing before `made`.
pub(crate) fn table_of(store: &FillStore, table: MadeTableId) -> Option<&TableSlot> {
    store
        .tables
        .get(usize::try_from(table.0).unwrap_or(usize::MAX))
        .filter(|slot| slot.stamp == store.stamp)
}

/// How many rows of a relation the file holds: every row the language wrote for it so far.
pub(crate) fn relation_rows(store: &FillStore, relation: RelationNumber) -> Count {
    rows_held(&store.relations, relation, store.stamp)
}

/// The rows of the site's source whose key columns hold `key`. Over a made table, made first, and over the node table: the
/// site's index, built the first time. Over a relation: with a key from a row, while the relation is not whole, the
/// language is asked for that key alone, once; with constants, or once it is whole, the site's index over the whole fill.
///
/// # Errors
///
/// The site's source could not be read.
pub(crate) fn matched(
    frame: &mut FillFrame<'_>,
    uses: &mut dyn PatternRows,
    site: IndexSite,
    key: &[CellWord],
    from: KeyFrom,
) -> Result<KeyRows, FilledError> {
    let programs = frame.programs;
    let source = site_source(programs.code, site_at(programs.code, site)?)?;
    let mut filled = Count(0);
    if let (ReadSource::Filled(number), KeyFrom::Row) = (source, from) {
        match key_asked(frame, site, number, key)? {
            KeyAnswer::Rows(rows) => return Ok(rows),
            KeyAnswer::Whole(written) => filled = written,
        }
    }
    let work = ensured(frame, uses, Reached::Index(site))?;
    let run = run_of(index_at(frame.store, site), key);
    Ok(KeyRows {
        rows: stretch_of(&frame.store.indexes, site, run),
        tag: tag_of(frame.file, source),
        filled: Count(filled.0.saturating_add(work.filled.0)),
        indexed: work.indexed,
    })
}

/// Every row of a source: a made table made first, a relation filled whole, the node table's elements of the extent.
///
/// # Errors
///
/// The source could not be read.
pub(crate) fn scanned(
    frame: &mut FillFrame<'_>,
    uses: &mut dyn PatternRows,
    source: ReadSource,
) -> Result<WholeRows, FilledError> {
    let work = match source {
        ReadSource::Nodes(_) => ensured(frame, uses, Reached::Nodes)?,
        ReadSource::Filled(number) => ensured(frame, uses, Reached::Relation(number))?,
        ReadSource::Made(table) => ensured(frame, uses, Reached::Table(table))?,
        ReadSource::Written(_) => {
            return Err(FilledError::Unbuilt {
                unbuilt: UnbuiltFill::Written,
            });
        }
    };
    Ok(WholeRows {
        rows: rows_in(frame.store, frame.nodes, source),
        tag: tag_of(frame.file, source),
        filled: work.filled,
    })
}

/// The cell of a row's column, by the row's handle: a made table's and a relation's as written; the node table's by the
/// column's code, a kind, a field and a text as the file's word of it. For one read; a reader of many rows of one
/// column of a made table or of a relation asks `reading::service::column_held` once and reads through `cell_held`.
#[inline]
pub(crate) fn cell_of(
    frame: &mut FillFrame<'_>,
    source: ReadSource,
    column: Short,
    row: CellWord,
) -> CellWord {
    match source {
        ReadSource::Filled(_) | ReadSource::Made(_) => {
            cell_held(frame.store, column_held(frame.store, source, column), row)
        }
        ReadSource::Nodes(_) => match node_column_of(column) {
            Some(named) => node_cell(
                frame.nodes,
                node_names(frame.programs, frame.text, frame.file),
                frame.words,
                named,
                row,
            ),
            None => NO_CELL,
        },
        ReadSource::Written(_) => NO_CELL,
    }
}

/// The handle of a row of a stretch: what a program binds for it.
#[inline]
pub(crate) fn handle_of(row: SourceRow, tag: CellWord) -> CellWord {
    tagged(CellWord(u64::from(row.0)), tag)
}
