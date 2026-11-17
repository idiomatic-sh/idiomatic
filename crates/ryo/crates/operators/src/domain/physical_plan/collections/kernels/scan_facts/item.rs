//! The `scan_facts` item of the `kernels` collection: a relation's rows read in place, as the facts lend them, never decoded
//! into a table; and the reading of rows in place, a relation's or a table's, one cell, one row or one chunk at a time.

#[cfg(test)]
use super::super::interface::{KernelsItem, Reach};
use super::super::scan_roots::item::{tree_cell, tree_count, tree_kinds, tree_row};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::types::fact_rows::value::FactRows;
use crate::domain::physical_plan::types::stream_scratch::value::{
    CellBound, ChunkRead, ChunkStop, RowChunk,
};
use crate::domain::physical_plan::types::{InputRows, PlanInputs};
#[cfg(test)]
use crate::domain::physical_plan::types::{RowSource, lowering::value::Lowering};
use crate::domain::table_texts::service::canonical_cell;
use crate::domain::table_texts::types::Texts;
use ryo_relations::domain::relations::service::from_bits;
use ryo_relations::domain::relations::types::{
    Bits, CellKind, ColumnsView, Integer, Relation, Value,
};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::{RowCount, RowIndex, Table};

/// `scan_facts`, one item of the `kernels` collection.
#[cfg(test)]
pub(crate) struct ScanFactsItem;

#[cfg(test)]
impl KernelsItem for ScanFactsItem {
    fn reached_by(&self, lowering: &Lowering) -> Reach {
        let runs = {
            matches!(lowering, Lowering::Lookup(_))
                || lowering
                    .sources()
                    .any(|source| matches!(source, RowSource::Facts(_)))
        };
        if runs { Reach::Runs } else { Reach::Skips }
    }
}

/// A relation's rows as the facts lend them: every relation's columns enter the executor here, and only whole.
///
/// # Errors
///
/// `NotRun` for a relation the facts refused: a fault, never empty rows. `ShapeMismatch` for columns fewer than the
/// relation's kinds, malformed rows no reader may read part of.
pub(crate) fn facts(
    inputs: &dyn PlanInputs,
    relation: Relation,
) -> Result<ColumnsView<'_>, PortableOperatorFault> {
    match inputs.facts(relation) {
        FactRows::Lent(view) if view.columns.len() < view.kinds.len() => {
            Err(PortableOperatorFault::ShapeMismatch)
        }
        FactRows::Lent(view) => Ok(view),
        FactRows::Refused => Err(PortableOperatorFault::NotRun),
    }
}

/// How many rows a relation's columns hold.
pub(crate) fn rows_of(view: &ColumnsView<'_>) -> RowCount {
    // The readable prefix: a column shorter than the others ends the rows every column holds.
    RowCount(
        view.columns
            .iter()
            .map(|column| column.0.len())
            .min()
            .unwrap_or(0),
    )
}

/// Row `row` of the columns, decoded into `into` one cell per column: the facts are read in place, never into a table.
pub(crate) fn decode(view: &ColumnsView<'_>, row: RowIndex, into: &mut Vec<Value>) {
    into.clear();
    // A column shorter than the row is a row of another width, which the filter refuses: no cell is made up for it.
    into.extend(
        view.kinds
            .iter()
            .zip(view.columns)
            .filter_map(|(kind, column)| {
                column
                    .0
                    .get(row.0)
                    .map(|bits| from_bits(*kind, Bits(*bits)))
            }),
    );
}

/// What each column of the rows holds.
pub(crate) fn kinds_of<'a>(rows: InputRows<'a>) -> &'a [CellKind] {
    match rows {
        InputRows::Facts(view) => view.kinds,
        InputRows::Tree(tree) => tree_kinds(tree),
        InputRows::Table(table) => &table.kinds,
        InputRows::Row(_, kinds) => kinds,
    }
}

/// How many rows there are.
pub(crate) fn count_of(rows: InputRows<'_>) -> RowCount {
    match rows {
        InputRows::Facts(view) => rows_of(view),
        InputRows::Tree(tree) => tree_count(tree),
        InputRows::Table(table) => table.count,
        InputRows::Row(..) => RowCount(1),
    }
}

/// The cell of row `row` in `column` of a relation's columns, read in place; none past the row.
pub(crate) fn facts_cell(
    view: &ColumnsView<'_>,
    row: RowIndex,
    column: ColumnIndex,
) -> Option<Value> {
    let at = usize::try_from(column.0).unwrap_or(usize::MAX);
    let kind = view.kinds.get(at)?;
    let bits = view.columns.get(at)?.0.get(row.0)?;
    Some(from_bits(*kind, Bits(*bits)))
}

/// The cells of row `row` of a table: none when the table does not hold every cell of the row (a row past its rows, or one
/// written short of its kinds), so a short row is read as no row, never into the next row's cells.
pub(crate) fn table_row(table: &Table, row: RowIndex) -> &[Value] {
    let width = table.kinds.len().max(1);
    table
        .cells
        .get(row.0 * width..row.0 * width + width)
        .unwrap_or_default()
}

/// The cell of row `row` in `column` of a table, as `table_row` reads the row; none past the row.
pub(crate) fn table_cell(table: &Table, row: RowIndex, column: ColumnIndex) -> Option<Value> {
    let at = usize::try_from(column.0).unwrap_or(usize::MAX);
    (at < table.kinds.len() && row.0 < table.count.0)
        .then(|| table_row(table, row).get(at).copied())
        .flatten()
}

/// The cell of row `row` in `column`, read in place; none past the row.
pub(crate) fn cell_of(rows: InputRows<'_>, row: RowIndex, column: ColumnIndex) -> Option<Value> {
    match rows {
        InputRows::Facts(view) => facts_cell(view, row, column),
        InputRows::Tree(tree) => tree_cell(tree, row, column),
        InputRows::Table(table) => table_cell(table, row, column),
        InputRows::Row(cells, _) => (row.0 == 0)
            .then(|| {
                cells
                    .get(usize::try_from(column.0).unwrap_or(usize::MAX))
                    .copied()
            })
            .flatten(),
    }
}

/// Reads the rows of `chunk` from its first unread one through `row`, which writes a row's rows to `out` and answers the
/// row's fault, until every row is read, a row faults (the rows it wrote before its fault stay), or `out` holds `bound`
/// cells or more after a row: a row's rows are never split across chunks.
///
/// # Errors
///
/// The hard fault `row` raises: the reader reads no more.
pub(crate) fn chunk_read(
    chunk: RowChunk<'_>,
    out: &mut Table,
    bound: CellBound,
    mut row: impl FnMut(
        RowIndex,
        &mut Table,
    ) -> Result<Option<PortableOperatorFault>, PortableOperatorFault>,
) -> Result<ChunkRead, PortableOperatorFault> {
    let count = count_of(chunk.rows).0;
    let mut at = chunk.from.0;
    while at < count {
        let fault = row(RowIndex(at), out)?;
        at += 1;
        let read = RowCount(at - chunk.from.0);
        if let Some(fault) = fault {
            return Ok(ChunkRead {
                read,
                stop: ChunkStop::Faulted(fault),
            });
        }
        if out.cells.len() >= bound.0 {
            return Ok(ChunkRead {
                read,
                stop: ChunkStop::Full,
            });
        }
    }
    Ok(ChunkRead {
        read: RowCount(at.saturating_sub(chunk.from.0)),
        stop: ChunkStop::Ended,
    })
}

/// Row `row`: a table's cells in place, a relation's decoded into `scratch`.
pub(crate) fn row_of<'r>(
    rows: InputRows<'r>,
    row: RowIndex,
    scratch: &'r mut Vec<Value>,
) -> &'r [Value] {
    match rows {
        InputRows::Facts(view) => {
            decode(view, row, scratch);
            scratch
        }
        InputRows::Tree(tree) => {
            tree_row(tree, row, scratch);
            scratch
        }
        InputRows::Table(table) => table_row(table, row),
        InputRows::Row(cells, _) => {
            if row.0 == 0 {
                cells
            } else {
                &[]
            }
        }
    }
}

/// Writes into `into` the canonical cells of `columns` of every row of `rows`, and numbers in `fresh` each text no symbol of
/// the file holds, in the order main's engine interned them: rows last first, columns in order.
///
/// # Errors
///
/// `NoSuchColumn` for a column past the rows; `ShapeMismatch` for a name with no text.
#[expect(
    clippy::indexing_slicing,
    reason = "every place is below the rows times the columns `into` was sized to just before"
)]
pub(crate) fn canonical_cells<'c>(
    rows: InputRows<'_>,
    columns: impl Iterator<Item = &'c ColumnIndex> + Clone,
    texts: &Texts<'_>,
    fresh: &mut Vec<Value>,
    into: &mut Vec<Value>,
) -> Result<(), PortableOperatorFault> {
    let width = columns.clone().count();
    let count = count_of(rows).0;
    into.clear();
    into.resize(count * width, Value::Number(Integer(0)));
    for at in (0..count).rev() {
        for (place, column) in columns.clone().enumerate() {
            let value = cell_of(rows, RowIndex(at), *column)
                .ok_or(PortableOperatorFault::NoSuchColumn { index: *column })?;
            into[at * width + place] = canonical_cell(value, texts, fresh)?;
        }
    }
    Ok(())
}
