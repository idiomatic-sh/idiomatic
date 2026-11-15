//! The `scan_facts` item of the `kernels` collection: a relation's rows read in place, as the facts lend them, never decoded
//! into a table; and the reading of rows in place, a relation's or a table's, one cell or one row at a time.

#[cfg(test)]
use super::super::interface::{KernelsItem, Reach};
use super::super::scan_roots::item::{tree_cell, tree_count, tree_kinds, tree_row};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::types::{FactRows, InputRows, PlanInputs};
#[cfg(test)]
use crate::domain::physical_plan::types::{Lowering, RowSource};
use ryo_relations::domain::relations::service::from_bits;
use ryo_relations::domain::relations::types::{Bits, CellKind, ColumnsView, Relation, Value};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::{RowCount, RowIndex};

/// `scan_facts`, one item of the `kernels` collection.
#[cfg(test)]
pub(crate) struct ScanFactsItem;

#[cfg(test)]
impl KernelsItem for ScanFactsItem {
    fn reached_by(&self, lowering: &Lowering) -> Reach {
        let runs = {
            matches!(lowering, Lowering::Lookup { .. })
                || super::super::scan_node::item::sources_of(lowering)
                    .any(|source| matches!(source, RowSource::Facts(_)))
        };
        if runs { Reach::Runs } else { Reach::Skips }
    }
}

/// A relation's rows as the facts lend them.
///
/// # Errors
///
/// `NotRun` for a relation the facts refused: a fault, never empty rows.
pub(crate) fn facts(
    inputs: &dyn PlanInputs,
    relation: Relation,
) -> Result<ColumnsView<'_>, PortableOperatorFault> {
    match inputs.facts(relation) {
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
    }
}

/// How many rows there are.
pub(crate) fn count_of(rows: InputRows<'_>) -> RowCount {
    match rows {
        InputRows::Facts(view) => rows_of(view),
        InputRows::Tree(tree) => tree_count(tree),
        InputRows::Table(table) => table.count,
    }
}

/// The cell of row `row` in `column`, read in place; none past the row.
pub(crate) fn cell_of(rows: InputRows<'_>, row: RowIndex, column: ColumnIndex) -> Option<Value> {
    let at = usize::try_from(column.0).unwrap_or(usize::MAX);
    match rows {
        InputRows::Facts(view) => {
            let kind = view.kinds.get(at)?;
            let bits = view.columns.get(at)?.0.get(row.0)?;
            Some(from_bits(*kind, Bits(*bits)))
        }
        InputRows::Tree(tree) => tree_cell(tree, row, column),
        InputRows::Table(table) => {
            let width = table.kinds.len();
            (at < width && row.0 < table.count.0)
                .then(|| table.cells.get(row.0 * width + at).copied())
                .flatten()
        }
    }
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
        InputRows::Table(table) => {
            let width = table.kinds.len().max(1);
            table
                .cells
                .get(row.0 * width..row.0 * width + width)
                .unwrap_or_default()
        }
    }
}
