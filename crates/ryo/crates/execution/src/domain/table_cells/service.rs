//! The `table_cells` service: reading some cells of a row, and the rows of a tree, with no IO and no ports.

use super::constants::KIND_IN_VOCABULARY;
use ryo_language::domain::elements::service::{
    end_at, field_name_at, kind_at, kind_names, ordinal_at, parent_at, start_at, text_at,
};
use ryo_language::domain::elements::types::Elements;

use ryo_relations::domain::relations::service::from_bits;
use ryo_relations::domain::relations::service::node_id;
use ryo_relations::domain::relations::types::{Bits, Integer, Position, Span, Value};
use ryo_relations::domain::terms::types::Symbol;

use crate::domain::table_cells::types::{CellsRead, ColumnPlaces, TreeForm, TreeRows};
use ryo_relations::domain::relations::types::{ColumnsView, FactColumn};

/// The name of the kind of the element at `at`, as the table's vocabulary names it.
///
/// # Panics
///
/// When the kind's number names no kind of the table's vocabulary: the table and its vocabulary are made together, so that is
/// a bug, never a kind with no name.
#[must_use]
#[expect(
    clippy::expect_used,
    reason = "the table's kinds are numbered in its own vocabulary; a number outside it is a bug that must crash, not answer no name"
)]
pub(crate) fn kind_name_at(elements: &Elements, at: Position) -> Symbol {
    kind_names(elements)
        .get(usize::from(kind_at(elements, at).0))
        .copied()
        .expect(KIND_IN_VOCABULARY)
}

/// The bytes the element at `at` covers.
#[must_use]
pub(crate) fn span_at(elements: &Elements, at: Position) -> Span {
    Span {
        start: start_at(elements, at),
        end: end_at(elements, at),
    }
}

/// The place of the element at `at` under its parent, as a number of a row.
#[must_use]
pub(crate) fn ordinal_number_at(elements: &Elements, at: Position) -> Integer {
    Integer(i64::from(ordinal_at(elements, at).0))
}

/// The row of the element at `at`.
#[must_use]
pub(crate) fn element_of(tree: &TreeRows<'_>, at: Position) -> [Value; 5] {
    let elements = tree.elements;
    [
        Value::Node(node_id(tree.file, at)),
        Value::Symbol(kind_name_at(elements, at)),
        Value::Symbol(text_at(elements, at)),
        Value::File(tree.file),
        Value::Span(span_at(elements, at)),
    ]
}

/// The edge to the element at `child`: its parent, itself, its place under the parent, its role, the file.
#[must_use]
pub(crate) fn edge_of(tree: &TreeRows<'_>, child: Position) -> [Value; 5] {
    [
        Value::Node(node_id(tree.file, parent_at(tree.elements, child))),
        Value::Node(node_id(tree.file, child)),
        Value::Number(ordinal_number_at(tree.elements, child)),
        Value::Symbol(field_name_at(tree.elements, child)),
        Value::File(tree.file),
    ]
}

/// Reads into `scratch`, which is as wide as the rows, the cells at `columns` of the stored row at
/// `index`, and nothing else: a row is decided on a few of its cells before it is read whole.
/// Answers whether the row is there.
pub(crate) fn read_cells(
    view: &ColumnsView<'_>,
    index: RowIndex,
    scratch: &mut [Value],
    columns: &ColumnPlaces,
) -> CellsRead {
    let read = columns.0.iter().all(|place| {
        let (Some(kind), Some(FactColumn(numbers)), Some(slot)) = (
            view.kinds.get(*place),
            view.columns.get(*place),
            scratch.get_mut(*place),
        ) else {
            return false;
        };
        numbers.get(index.0).is_some_and(|number| {
            *slot = from_bits(*kind, Bits(*number));
            true
        })
    });
    if read {
        CellsRead::Read
    } else {
        CellsRead::Missing
    }
}

/// Reads into `scratch`, which is as wide as the rows, the cells at `columns` of the row at `index` of
/// the elements of a tree, and nothing else. Answers whether the row is there.
pub(crate) fn read_tree_cells(
    tree: &TreeRows<'_>,
    index: RowIndex,
    scratch: &mut [Value],
    columns: &ColumnPlaces,
) -> CellsRead {
    let first = usize::from(tree.form == TreeForm::Edges);
    let Ok(at) = u32::try_from(index.0 + first) else {
        return CellsRead::Missing;
    };
    if usize::try_from(at).map_or(true, |place| place >= tree.elements.kind.len()) {
        return CellsRead::Missing;
    }
    let at = Position(at);
    let elements = tree.elements;
    let read = columns.0.iter().all(|place| {
        let Some(slot) = scratch.get_mut(*place) else {
            return false;
        };
        *slot = match (tree.form, *place) {
            (TreeForm::Elements, 0) | (TreeForm::Edges, 1) => Value::Node(node_id(tree.file, at)),
            (TreeForm::Elements, 1) => Value::Symbol(kind_name_at(elements, at)),
            (TreeForm::Elements, 2) => Value::Symbol(text_at(elements, at)),
            (TreeForm::Elements, 3) | (TreeForm::Edges, 4) => Value::File(tree.file),
            (TreeForm::Elements, 4) => Value::Span(span_at(elements, at)),
            (TreeForm::Edges, 0) => Value::Node(node_id(tree.file, parent_at(elements, at))),
            (TreeForm::Edges, 2) => Value::Number(ordinal_number_at(elements, at)),
            (TreeForm::Edges, 3) => Value::Symbol(field_name_at(elements, at)),
            _ => return false,
        };
        true
    });
    if read {
        CellsRead::Read
    } else {
        CellsRead::Missing
    }
}

use ryo_values::domain::rows::types::RowIndex;

/// A native reader exists only through the common native-relation catalog.
pub(crate) fn native_form(
    relation: ryo_relations::domain::relations::types::Relation,
) -> Option<TreeForm> {
    super::constants::NATIVE_RELATIONS
        .iter()
        .find_map(|(native, form)| (*native == relation).then_some(*form))
}

/// Native relation delivery and per-file admission share this exact catalog.
pub(crate) fn native_relations()
-> &'static [(ryo_relations::domain::relations::types::Relation, TreeForm)] {
    super::constants::NATIVE_RELATIONS
}
