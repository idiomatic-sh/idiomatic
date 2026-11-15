//! The `table_model` service: free functions named by verb over tables, with no IO and no ports.


use ryo_language::domain::elements::service::real_count;
use ryo_relations::domain::relations::types::{
    CellKind, Value,
};

use super::constants::MATERIALIZED;
use super::types::RepresentationKind;
use super::types::registered_representation::value::RegisteredRepresentation;
use super::types::{RepresentationInput, Rows};
use ryo_operators::domain::tree_cells::types::{TreeForm, TreeRows};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::errors::RowsError;

use ryo_values::domain::rows::service::{positions, push_row, reset};

/// What each column of the rows holds.
#[must_use]
pub fn kinds_of<'a>(rows: &'a Rows<'_>) -> &'a [CellKind] {
    match rows {
        Rows::Stored(view) => view.kinds,
        Rows::Tree(TreeRows {
            form: TreeForm::Edges,
            ..
        }) => &EDGE_KINDS,
        Rows::Tree(TreeRows {
            form: TreeForm::Elements,
            ..
        }) => &ELEMENT_KINDS,
        Rows::Computed(table) => &table.kinds,
        Rows::Compact(table) => ryo_values::domain::compact_table::service::compact_kinds(table),
    }
}

/// How many rows there are.
#[must_use]
pub fn count_of(rows: &Rows<'_>) -> RowCount {
    match rows {
        Rows::Stored(view) => RowCount(view.columns.first().map_or(0, |column| column.0.len())),
        Rows::Computed(table) => table.count,
        Rows::Compact(table) => ryo_values::domain::compact_table::service::compact_rows(table),
        Rows::Tree(tree) => RowCount(
            real_count(tree.elements)
                .0
                .saturating_sub(usize::from(tree.form == TreeForm::Edges)),
        ),
    }
}

/// What the columns of the edges of a tree hold: the parent, the child, the child's place under
/// its parent, and the file.
const EDGE_KINDS: [CellKind; 5] = [
    CellKind::Node,
    CellKind::Node,
    CellKind::Number,
    CellKind::Symbol,
    CellKind::File,
];

/// What the columns of the elements of a tree hold: the element, its kind, its text, the file, and where it is.
const ELEMENT_KINDS: [CellKind; 5] = [
    CellKind::Node,
    CellKind::Symbol,
    CellKind::Symbol,
    CellKind::File,
    CellKind::Span,
];

/// The row at `index`: a computed row where it is, a stored row read into `scratch`, which keeps
/// its room from row to row.
pub fn read_row<'s>(
    rows: &'s Rows<'s>,
    index: RowIndex,
    scratch: &'s mut Vec<Value>,
) -> Option<&'s [Value]> {
    borrowed(rows).read(index, scratch)
}

/// Copies the rows into `into`, keeping the room `into` has.
pub fn materialize(rows: &Rows<'_>, into: &mut Table, scratch: &mut Vec<Value>) {
    reset(into, kinds_of(rows));
    for index in positions(count_of(rows)) {
        if let Some(row) = read_row(rows, index, scratch) {
            push_row(into, row);
        }
    }
    tracing::trace!(target: "ryo::diagnostic::selection", event = MATERIALIZED,
        compact_source = matches!(rows, Rows::Compact(_)),
        rows = into.count.0, columns = into.kinds.len(),
        cell_bytes = into.cells.len().saturating_mul(size_of::<ryo_relations::domain::relations::types::Value>()),
        capacity_bytes = into.cells.capacity().saturating_mul(size_of::<ryo_relations::domain::relations::types::Value>()));
}


/// Whether a table whose columns hold `kinds` has every one of `columns`.
///
/// # Errors
///
/// `NoSuchColumn` naming the first it lacks.
pub fn check_columns(kinds: &[CellKind], columns: &[ColumnIndex]) -> Result<(), RowsError> {
    match columns
        .iter()
        .find(|column| usize::try_from(column.0).map_or(true, |place| place >= kinds.len()))
    {
        Some(missing) => Err(RowsError::NoSuchColumn { index: missing.0 }),
        None => Ok(()),
    }
}






use ryo_values::domain::rows::types::{RowCount, RowIndex, Table};

/// The classified catalog value that reads rows of `kind`.
pub fn registered(kind: RepresentationKind) -> RegisteredRepresentation {
    RegisteredRepresentation::new(kind)
}

/// The representation `input` is held in.
pub fn kind(input: RepresentationInput<'_>) -> RepresentationKind {
    RegisteredRepresentation::kind(input)
}

/// The rows as a physical representation reads them.
pub(crate) fn borrowed<'a>(rows: &'a Rows<'a>) -> RepresentationInput<'a> {
    RepresentationInput::from(rows)
}
