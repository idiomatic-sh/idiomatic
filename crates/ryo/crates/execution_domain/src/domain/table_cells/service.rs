//! The `table_cells` service: reading some cells of a row, and the rows of a tree, with no IO and no ports.

use ryo_operators::domain::tree_cells::service::tree_value;
use ryo_operators::domain::tree_cells::types::{TreeForm, TreeRows};
use ryo_relations::domain::relations::types::{Position, Value};
use ryo_values::domain::plan_parts::types::ColumnIndex;

use crate::domain::table_cells::types::{CellsRead, ColumnPlaces};

/// How many columns a row of a tree has.
const TREE_WIDTH: usize = 5;

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
    let read = columns.0.iter().all(|place| {
        let (Some(slot), Ok(column)) = (scratch.get_mut(*place), u32::try_from(*place)) else {
            return false;
        };
        if *place >= TREE_WIDTH {
            return false;
        }
        *slot = tree_value(tree, at, ColumnIndex(column));
        true
    });
    if read {
        CellsRead::Read
    } else {
        CellsRead::Missing
    }
}

use ryo_values::domain::rows::types::RowIndex;

/// The physical column of the tree a view's logical column reads: a view names only the columns it moves,
/// so a column it does not name is the tree's own.
pub(crate) fn source_column(
    view: &[ColumnIndex],
    column: ColumnIndex,
) -> ColumnIndex {
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
                TreeForm::Edges if key == ColumnIndex(0) => {
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
                let element = tree_value(&TreeRows { form: TreeForm::Elements, ..*tree }, *at, ColumnIndex(4));
                element == Value::Span(span)
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
