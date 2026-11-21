//! The services of the `reading` part: what is there for the file, read and never made. A word's source, a site's source,
//! a column of a source as a loop reads it, a built index, a pipeline's words.

use crate::domain::code::constants::{OWN_TREE, SOURCE};
use crate::domain::code::types::{
    CodeEntry, CodeWord, IndexCode, IndexSite, Op, PoolAt, ProgramCode, RelationNumber, Short,
    WordFlag, WordSlot,
};
use crate::domain::filled::errors::FilledError;
use crate::domain::filled::service::whole_rows;
use crate::domain::filled::types::MissingCode;
use crate::domain::indexes::reading::service::read;
use crate::domain::indexes::types::{ColumnView, FixedKeys, IndexRead, RowRun, SourceRow};
use crate::domain::made_tables::columns::types::{CellsLie, HeldCells, HeldColumn, RowColumn};
use crate::domain::made_tables::service::{file_of, key_mask, place_of};
use crate::domain::made_tables::types::{FillStore, MadeState, ReadSource, RuleSetView, TableSlot};
use crate::domain::stages::nodes::service::node_tag;
use crate::domain::stages::service::row_of;
use crate::domain::stages::types::NodeNames;
use crate::domain::steps::types::MadeTableId;
use ryo_language::domain::facts::types::CellWord;
use ryo_language::domain::node_table::service::{element_count, own_end};
use ryo_language::domain::node_table::types::NodeTable;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_relations::domain::catalog::service::node_extent;
use ryo_relations::domain::relations::types::{Count, FileId, NodeExtent, Relation};

/// The source a word names: the source its flag's bits say, the file's own tree alone under the own-tree bit, and the
/// relation's or the table's number.
#[must_use]
pub(crate) const fn source_of(flag: WordFlag, read: WordSlot) -> ReadSource {
    match flag.0 & SOURCE.0 {
        0 => ReadSource::Nodes(if flag.0 & OWN_TREE.0 == 0 {
            NodeExtent::Embedded
        } else {
            NodeExtent::Own
        }),
        1 => ReadSource::Filled(RelationNumber(read.0)),
        2 => ReadSource::Made(MadeTableId(read.0)),
        3..=u8::MAX => ReadSource::Written(MadeTableId(read.0)),
    }
}

/// The relation the code numbers `number`.
///
/// # Errors
///
/// The code reads no relation of that number.
pub(crate) fn relation_at(
    code: &ProgramCode,
    number: RelationNumber,
) -> Result<Relation, FilledError> {
    code.relations
        .get(usize::try_from(number.0).unwrap_or(usize::MAX))
        .copied()
        .ok_or(FilledError::Code {
            named: MissingCode::Relation,
        })
}

/// What a site's index is over.
///
/// # Errors
///
/// The code holds no such site.
pub(crate) fn site_at(code: &ProgramCode, site: IndexSite) -> Result<IndexCode, FilledError> {
    code.indexes
        .get(usize::try_from(site.0).unwrap_or(usize::MAX))
        .copied()
        .ok_or(FilledError::Code {
            named: MissingCode::Site,
        })
}

/// The source of a site's index. Over the node table, the extent is its relation's: the catalog's, which the encoding
/// reads for a word's own-tree bit.
///
/// # Errors
///
/// The site names a source the code does not hold.
pub(crate) fn site_source(code: &ProgramCode, index: IndexCode) -> Result<ReadSource, FilledError> {
    match index.source {
        Short(0) => {
            let relation = relation_at(code, RelationNumber(index.number.0))?;
            Ok(ReadSource::Nodes(match node_extent(relation) {
                Some(NodeExtent::Own) => NodeExtent::Own,
                Some(NodeExtent::Embedded) | None => NodeExtent::Embedded,
            }))
        }
        Short(1) => Ok(ReadSource::Filled(RelationNumber(index.number.0))),
        Short(2) => Ok(ReadSource::Made(MadeTableId(index.number.0))),
        Short(3..=u16::MAX) => Err(FilledError::Code {
            named: MissingCode::Site,
        }),
    }
}

/// The bits every handle of a source's rows carries besides the row's number: the file, of the node table's; nothing of
/// another source.
#[must_use]
pub(crate) fn tag_of(file: FileId, source: ReadSource) -> CellWord {
    match source {
        ReadSource::Nodes(_) => node_tag(file),
        ReadSource::Filled(_) | ReadSource::Made(_) | ReadSource::Written(_) => CellWord(0),
    }
}

/// The site a `Latest` reads: the first slot of its pool record, before the key's operands.
///
/// # Errors
///
/// The pool does not reach the record.
pub(crate) fn latest_site(code: &ProgramCode, record: PoolAt) -> Result<IndexSite, FilledError> {
    code.pool
        .get(usize::try_from(record.0).unwrap_or(usize::MAX))
        .map(|slot| IndexSite(slot.0))
        .ok_or(FilledError::Code {
            named: MissingCode::Pool,
        })
}

/// A table's slot when the table is made for the file.
pub(crate) fn made_slot(store: &FillStore, table: MadeTableId) -> Option<&TableSlot> {
    store
        .tables
        .get(usize::try_from(table.0).unwrap_or(usize::MAX))
        .filter(|slot| slot.stamp == store.stamp && slot.state == MadeState::Made)
}

/// The handles of every row of a source as it is for the file: a made table's rows, a relation's whole fill (nothing
/// before it is whole), the node table's elements of the extent.
pub(crate) fn rows_in(store: &FillStore, nodes: &NodeTable, source: ReadSource) -> RowRun {
    match source {
        ReadSource::Nodes(extent) => RowRun {
            start: Count(0),
            end: match extent {
                NodeExtent::Own => Count(usize::try_from(own_end(nodes).0).unwrap_or(usize::MAX)),
                NodeExtent::Embedded => element_count(nodes),
            },
        },
        ReadSource::Filled(number) => whole_rows(&store.relations, number, store.stamp),
        ReadSource::Made(table) => RowRun {
            start: Count(0),
            end: made_slot(store, table).map_or(Count(0), |slot| slot.rows.count),
        },
        ReadSource::Written(_) => RowRun::default(),
    }
}

/// Where a column of a made table or of a filled relation lies, for the file: asked once for a table and a column, and
/// kept by the reader, which indexes the cells itself (`cells_held`, `cell_held`). Nothing for the node table, whose
/// cells are made when read (`stages::nodes`), for a table not made for the file, and for a column the source has not.
pub(crate) fn column_held(store: &FillStore, source: ReadSource, at: Short) -> HeldColumn {
    let nothing = HeldColumn {
        cells: HeldCells::Nothing,
        first: Count(0),
        stride: Count(1),
        end: Count(0),
    };
    match source {
        ReadSource::Filled(number) => {
            let filled = store
                .relations
                .0
                .get(usize::try_from(number.0).unwrap_or(usize::MAX))
                .filter(|slot| slot.stamp == store.stamp)
                .is_some_and(|slot| slot.columns.0.len() > usize::from(at.0));
            if filled {
                HeldColumn {
                    cells: HeldCells::Relation(number, at),
                    first: Count(0),
                    stride: Count(1),
                    end: Count(usize::MAX),
                }
            } else {
                nothing
            }
        }
        ReadSource::Made(table) => match made_slot(store, table) {
            Some(slot) => match (
                place_of(&slot.rows, Count(usize::from(at.0))),
                slot.rows.lies,
            ) {
                // A made table's cells are a stretch of the arena: the column is read there, with no slot looked up.
                (RowColumn::Kept(first), CellsLie::Arena(start)) => HeldColumn {
                    cells: HeldCells::Arena,
                    first: Count(start.0.saturating_add(first.0)),
                    stride: slot.rows.width,
                    end: Count(
                        start
                            .0
                            .saturating_add(slot.rows.count.0.saturating_mul(slot.rows.width.0)),
                    ),
                },
                (RowColumn::Kept(first), CellsLie::Own) => HeldColumn {
                    cells: HeldCells::Table(table),
                    first,
                    stride: slot.rows.width,
                    end: Count(usize::MAX),
                },
                (RowColumn::File, CellsLie::Arena(_) | CellsLie::Own) => HeldColumn {
                    cells: HeldCells::TableFile(table),
                    first: Count(0),
                    stride: Count(0),
                    end: Count(1),
                },
            },
            None => nothing,
        },
        ReadSource::Nodes(_) | ReadSource::Written(_) => nothing,
    }
}

/// The cells a held column lies in, as they are now: read where they are, with no test of the file, which
/// `column_held` made when the column was asked.
#[inline(always)]
pub(crate) fn cells_held(store: &FillStore, held: HeldCells) -> &[CellWord] {
    match held {
        HeldCells::Arena => &store.table_cells,
        HeldCells::Table(table) => match store
            .tables
            .get(usize::try_from(table.0).unwrap_or(usize::MAX))
        {
            Some(slot) => &slot.rows.cells,
            None => &[],
        },
        HeldCells::TableFile(table) => match store
            .tables
            .get(usize::try_from(table.0).unwrap_or(usize::MAX))
        {
            Some(slot) => &slot.rows.file.0,
            None => &[],
        },
        HeldCells::Relation(number, at) => match store
            .relations
            .0
            .get(usize::try_from(number.0).unwrap_or(usize::MAX))
            .and_then(|slot| slot.columns.0.get(usize::from(at.0)))
        {
            Some(cells) => &cells.0,
            None => &[],
        },
        HeldCells::Nothing => &[],
    }
}

/// The cell of a row in a held column, by the row's handle; the zero cell past the column's end. One test of the
/// place against the end of the source's own cells: what does not change from row to row (the cells, their end) is
/// found once by a reader that reads many rows.
#[inline]
pub(crate) fn cell_held(store: &FillStore, column: HeldColumn, row: CellWord) -> CellWord {
    let cells = cells_held(store, column.cells);
    let own = cells
        .get(..column.end.0.min(cells.len()))
        .unwrap_or_default();
    let at = column
        .first
        .0
        .wrapping_add(row_of(row).0.wrapping_mul(column.stride.0));
    match own.get(at) {
        Some(cell) => *cell,
        None => CellWord(0),
    }
}

/// A column of a relation or of a made table as a loop reads it by a row's handle; no cell for the node table, whose
/// columns are read by `stages::nodes`, and none for a source that is not there. The held column's cells, lent.
pub(crate) fn view_of(store: &FillStore, source: ReadSource, at: Short) -> ColumnView<'_> {
    let held = column_held(store, source, at);
    let cells = cells_held(store, held.cells);
    ColumnView {
        // No cell past the source's own: a made table's stretch of the arena ends where the next table's starts.
        cells: cells.get(..held.end.0.min(cells.len())).unwrap_or_default(),
        first: held.first,
        stride: held.stride,
    }
}

/// Which columns of a key over a source hold one cell in every row, and that cell: a made table's file columns, of
/// which it keeps no cell; none of a relation.
pub(crate) fn fixed_of(store: &FillStore, source: ReadSource, columns: &[WordSlot]) -> FixedKeys {
    match source {
        ReadSource::Made(table) => {
            made_slot(store, table).map_or(FixedKeys::default(), |slot| FixedKeys {
                skipped: key_mask(
                    &slot.rows,
                    columns
                        .iter()
                        .map(|column| Count(usize::try_from(column.0).unwrap_or(usize::MAX))),
                ),
                cell: file_of(&slot.rows),
            })
        }
        ReadSource::Filled(_) | ReadSource::Nodes(_) | ReadSource::Written(_) => {
            FixedKeys::default()
        }
    }
}

/// The built index of a site, lent for reading; an index of no row for a site not built for the file.
#[inline]
pub(crate) fn index_at(store: &FillStore, site: IndexSite) -> IndexRead<'_> {
    read(&store.indexes, site)
}

/// The rows of a stretch of the store's rows: each a handle of its source.
#[inline]
pub(crate) fn rows_at(store: &FillStore, run: RowRun) -> &[SourceRow] {
    store
        .indexes
        .rows
        .get(run.start.0..run.end.0)
        .unwrap_or_default()
}

/// The words of the pipeline that starts at `entry`: its stages and, last, its write.
///
/// # Errors
///
/// The code holds no word there, or no write after it.
pub(crate) fn pipeline_words(
    code: &ProgramCode,
    entry: CodeEntry,
) -> Result<&[CodeWord], FilledError> {
    let missing = FilledError::Code {
        named: MissingCode::Word,
    };
    let from = code
        .words
        .get(usize::try_from(entry.0).unwrap_or(usize::MAX)..)
        .ok_or(missing.clone())?;
    let write = from
        .iter()
        .position(|word| word.op == Op::StageWrite)
        .ok_or(missing.clone())?;
    from.get(..=write).ok_or(missing)
}

/// What a node row's cells are read from besides the table: the words of the kinds' and the fields' names, the file's
/// bytes and the file.
pub(crate) fn node_names<'a>(
    programs: RuleSetView<'a>,
    text: ByteSlice<'a>,
    source: Option<&'a str>,
    file: FileId,
) -> NodeNames<'a> {
    NodeNames {
        kinds: programs.kinds,
        fields: programs.fields,
        text,
        source,
        file,
    }
}
