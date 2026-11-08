//! The `table_cells` service: reading some cells of a row, and the rows of a tree, with no IO and no ports.

use super::constants::KIND_IN_VOCABULARY;
use ryo_language::domain::elements::service::{
    end_at, field_name_at, kind_at, kind_names, ordinal_at, parent_at, start_at, text_at,
};
use ryo_language::domain::elements::types::Elements;

use ryo_relations::domain::relations::service::node_id;
use ryo_relations::domain::relations::types::{Integer, Position, Span, Value};
use ryo_relations::domain::terms::types::Symbol;

use crate::domain::table_cells::types::{CellsRead, ColumnPlaces, TreeForm, TreeRows};

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
pub fn native_form(
    relation: ryo_relations::domain::relations::types::Relation,
) -> Option<TreeForm> {
    super::constants::NATIVE_RELATIONS
        .iter()
        .find_map(|(native, form)| (*native == relation).then_some(*form))
}

/// One cell of the element at `at`, in the column order of the relation the tree's rows stand for; a column
/// past them is the lowered flag.
pub(crate) fn tree_value(
    tree: &TreeRows<'_>,
    at: Position,
    column: ryo_values::domain::plan_parts::types::ColumnIndex,
) -> Value {
    let elements = tree.elements;
    if tree.form == TreeForm::Edges {
        return match column.0 {
            0 => Value::Node(node_id(tree.file, parent_at(elements, at))),
            1 => Value::Node(node_id(tree.file, at)),
            2 => Value::Number(ordinal_number_at(elements, at)),
            3 => Value::Symbol(field_name_at(elements, at)),
            4 => Value::File(tree.file),
            _ => Value::Flag(ryo_relations::domain::relations::types::Flag::Lowered),
        };
    }
    match column.0 {
        0 => Value::Node(node_id(tree.file, at)),
        1 => Value::Symbol(kind_name_at(elements, at)),
        2 => Value::Symbol(text_at(elements, at)),
        3 => Value::File(tree.file),
        4 => Value::Span(span_at(elements, at)),
        _ => Value::Flag(ryo_relations::domain::relations::types::Flag::Lowered),
    }
}

/// The physical column of the tree a view's logical column reads: a view names only the columns it moves,
/// so a column it does not name is the tree's own.
pub(crate) fn source_column(
    view: &[ryo_values::domain::plan_parts::types::ColumnIndex],
    column: ryo_values::domain::plan_parts::types::ColumnIndex,
) -> ryo_values::domain::plan_parts::types::ColumnIndex {
    match view.get(usize::try_from(column.0).unwrap_or(usize::MAX)) {
        Some(moved) => *moved,
        None => column,
    }
}

/// Pushes onto `found` the elements a left row finds in the tree: by the node it holds (the element itself,
/// the child an edge leads to, or, when the key is the parent of the edges, the children of the node), or by
/// the stretch of a file it holds (the elements that cover exactly that stretch). A node or stretch of
/// another file finds none.
///
/// # Errors
///
/// `NoSuchColumn` when the row lacks a column the finding reads.
pub(crate) fn find_native(
    tree: &TreeRows<'_>,
    left_row: &[Value],
    finding: ryo_derived::domain::native::types::NativeFinding,
    found: &mut Vec<Position>,
) -> Result<(), crate::domain::operating::errors::OperatingError> {
    use ryo_derived::domain::native::types::NativeFinding;
    use ryo_values::domain::rows::service::cell;
    let span = match finding {
        NativeFinding::Node(left_key, key) => {
            let Value::Node(start) = cell(left_row, left_key)? else {
                return Ok(());
            };
            let at = ryo_relations::domain::relations::service::node_position(start);
            if ryo_relations::domain::relations::service::node_file(start) != tree.file
                || usize::try_from(at.0).map_or(true, |place| place >= tree.elements.kind.len())
            {
                return Ok(());
            }
            // An edge has a parent but the root: the edges of the root's children are found by it, no edge
            // leads to the root.
            match tree.form {
                TreeForm::Edges if key == ryo_values::domain::plan_parts::types::ColumnIndex(0) => {
                    found.extend(ryo_language::domain::elements::service::children(
                        tree.elements,
                        at,
                    ));
                }
                TreeForm::Edges if at.0 == 0 => {}
                TreeForm::Edges | TreeForm::Elements => found.push(at),
            }
            return Ok(());
        }
        NativeFinding::Stretch(file, stretch) => {
            let (Value::File(of), Value::Span(span)) =
                (cell(left_row, file)?, cell(left_row, stretch)?)
            else {
                return Ok(());
            };
            if of != tree.file {
                return Ok(());
            }
            span
        }
        NativeFinding::Span(stretch) => {
            let Value::Span(span) = cell(left_row, stretch)? else {
                return Ok(());
            };
            span
        }
    };
    if ryo_language::domain::elements::service::source_order(tree.elements)
        == ryo_language::domain::elements::types::SourceOrder::Reordered
    {
        found.extend(
            ryo_language::domain::elements::service::positions(tree.elements).filter(|at| {
                let element = span_at(tree.elements, *at);
                element.start == span.start && element.end == span.end
            }),
        );
        return Ok(());
    }
    let starts = &tree.elements.start;
    let from = starts.partition_point(|start| *start < span.start);
    for place in from..starts.len() {
        if starts.get(place) != Some(&span.start) {
            break;
        }
        if tree.elements.end.get(place) == Some(&span.end) {
            found.push(Position(u32::try_from(place).unwrap_or(u32::MAX)));
        }
    }
    Ok(())
}
