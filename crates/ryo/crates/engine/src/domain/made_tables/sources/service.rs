//! The services of the `sources` part: what a program's step or a pipeline's stage asks of the fill side. Every source is
//! read through here: a made table is made on its first read and once, a relation is asked of the language whole or for
//! one key, an index is built by the columns asked the first time it is read.

use crate::domain::code::constants::{OWN_TREE, SOURCE};
use crate::domain::code::types::{IndexSite, Short, WordFlag, WordSlot};
use crate::domain::filled::errors::FilledError;
use crate::domain::filled::types::UnbuiltFill;
use crate::domain::indexes::types::{RowRun, SourceRow};
use crate::domain::made_tables::types::{
    FillFrame, FillStore, KeyFrom, MadeNow, KeyRows, PatternRows, ReadSource, WholeRows, TableSlot,
};
use crate::domain::steps::types::MadeTableId;
use ryo_language::domain::facts::types::CellWord;

/// Starts a file: every relation, index and table of the file before is unset, its buffers kept.
pub(crate) fn next_file(store: &mut FillStore) {
    store.stamp.0 = store.stamp.0.wrapping_add(1);
}

/// The source a word names: its flag's source bits and own-tree bit, and the relation's or the table's number.
pub(crate) const fn source_of(flag: WordFlag, read: WordSlot) -> ReadSource {
    let _unbuilt = (flag.0 & SOURCE.0, flag.0 & OWN_TREE.0);
    ReadSource::Made(MadeTableId(read.0))
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
    let _unbuilt = (frame.store.stamp, table);
    let _held: Option<&mut dyn PatternRows> = Some(uses);
    Err(FilledError::Unbuilt {
        unbuilt: UnbuiltFill::Written,
    })
}

/// A table's slot: its rows, its work, what it reached; nothing before `made`.
pub(crate) fn table_of(store: &FillStore, table: MadeTableId) -> Option<&TableSlot> {
    store
        .tables
        .get(usize::try_from(table.0).unwrap_or(usize::MAX))
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
    let _unbuilt = (frame.store.stamp, site, key, from);
    let _held: Option<&mut dyn PatternRows> = Some(uses);
    Err(FilledError::Unbuilt {
        unbuilt: UnbuiltFill::Written,
    })
}

/// The rows of a stretch `matched` answered: each a handle of its source.
pub(crate) fn rows_at(store: &FillStore, run: RowRun) -> &[SourceRow] {
    store.indexes.rows.get(run.start.0..run.end.0).unwrap_or(&[])
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
    let _unbuilt = (frame.store.stamp, source);
    let _held: Option<&mut dyn PatternRows> = Some(uses);
    Err(FilledError::Unbuilt {
        unbuilt: UnbuiltFill::Written,
    })
}

/// The cell of a row's column, by the row's handle: a made table's and a relation's as written; the node table's by the
/// column's code, a kind, a field and a text as the file's word of it.
pub(crate) fn cell_of(
    frame: &mut FillFrame<'_>,
    source: ReadSource,
    column: Short,
    row: CellWord,
) -> CellWord {
    let _unbuilt = (frame.store.stamp, source, column);
    row
}
