//! The `scan_roots` item of the `kernels` collection: a tree relation's rows read in place from the open file's elements, at
//! the tree nodes the engine names for the reading node, in tree order; one cell at a time, never decoded into columns.

use crate::domain::physical_plan::types::keyed_rows::value::{IndexedRow, NO_ROW};
use crate::domain::physical_plan::types::{PlanInputs, PlanNodeId, TreeRead, TreeRoots};
use crate::domain::tree_cells::service::{native_form, tree_table, tree_value, with_rows};
use crate::domain::tree_cells::types::{TreeForm, TreeRows};
use ryo_language::domain::elements::service::{children, element_count};
use ryo_language::domain::elements::types::Elements;
use ryo_relations::domain::relations::service::{kinds, node_file, node_id, node_position};
use ryo_relations::domain::relations::types::{CellKind, NodeId, Relation, Value};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::{RowCount, RowIndex, Table};



/// The tree nodes whose rows of tree relation `relation` `node` reads: the nodes the engine names for the node, in tree
/// order. The root has no edge, so the edges' rows skip it.
pub(crate) fn rooted<'a>(
    inputs: &'a dyn PlanInputs,
    node: PlanNodeId,
    relation: Relation,
) -> TreeRoots<'a> {
    let form = native_form(relation).unwrap_or(TreeForm::Elements);
    TreeRoots {
        form,
        kinds: kinds(relation),
        candidates: with_rows(form, inputs.roots(node).0),
    }
}

/// The rows of `roots` read from the open file's `elements`.
pub(crate) fn tree_read<'a>(roots: TreeRoots<'a>, elements: &'a Elements) -> TreeRead<'a> {
    TreeRead {
        form: roots.form,
        kinds: roots.kinds,
        elements,
        candidates: roots.candidates,
    }
}

/// The rows of tree relation `relation` that `node` reads: the open file's elements at the tree nodes the engine names for
/// the node, in tree order. The root has no edge, so the edges' rows skip it.
pub(crate) fn roots<'a>(
    inputs: &'a dyn PlanInputs,
    elements: &'a Elements,
    node: PlanNodeId,
    relation: Relation,
) -> TreeRead<'a> {
    tree_read(rooted(inputs, node, relation), elements)
}

/// Appends the nodes of the tree read's rows whose node column `column` holds `key`, in tree order: an element by its id, an
/// edge by its child (none for the root), the edges of a parent's children. A row's cell is the one `tree_value` reads, so a
/// row is found exactly when an index of the read keyed by `column` holds it under `key`; the walk only names the rows that
/// may: the key's own node and its children. A key that is no node, a node of another file than the read's, or a position
/// past the file's elements finds none.
pub(crate) fn keyed_roots(
    tree: &TreeRead<'_>,
    column: ColumnIndex,
    key: Value,
    found: &mut Vec<NodeId>,
) {
    // The open file is the file of the read's first row; an empty read finds nothing.
    let (Value::Node(node), Some(first)) = (key, tree.candidates.first()) else {
        return;
    };
    let file = node_file(*first);
    let at = node_position(node);
    if node_file(node) != file
        || usize::try_from(at.0).map_or(true, |place| place >= element_count(tree.elements).0)
    {
        return;
    }
    let rows = TreeRows {
        form: tree.form,
        elements: tree.elements,
        file,
    };
    for each in std::iter::once(at).chain(children(tree.elements, at)) {
        let row = node_id(file, each);
        if tree_value(&rows, each, column) == key && tree.candidates.binary_search(&row).is_ok() {
            found.push(row);
        }
    }
}

/// The found row after `row` among `found` rows, or the first when `row` is none; `NO_ROW` past the last: the rows a walk
/// found, read in order as an index's run of a key is.
pub(crate) fn next_found(row: Option<IndexedRow>, found: RowCount) -> IndexedRow {
    let next = row.map_or(RowIndex(0), |row| {
        RowIndex(usize::try_from(row.0).map_or(usize::MAX, |at| at.saturating_add(1)))
    });
    if next.0 < found.0 {
        IndexedRow(u32::try_from(next.0).unwrap_or(u32::MAX))
    } else {
        NO_ROW
    }
}

/// What each column of a tree relation's rows holds.
pub(crate) const fn tree_kinds<'a>(tree: &TreeRead<'a>) -> &'a [CellKind] {
    tree.kinds
}

/// How many rows the tree read holds.
pub(crate) const fn tree_count(tree: &TreeRead<'_>) -> RowCount {
    RowCount(tree.candidates.len())
}

/// The cell of row `row` in `column`, read from the elements; none past the row.
pub(crate) fn tree_cell(tree: &TreeRead<'_>, row: RowIndex, column: ColumnIndex) -> Option<Value> {
    let node = *tree.candidates.get(row.0)?;
    (usize::try_from(column.0).unwrap_or(usize::MAX) < tree.kinds.len()).then(|| {
        let rows = TreeRows {
            form: tree.form,
            elements: tree.elements,
            file: node_file(node),
        };
        tree_value(&rows, node_position(node), column)
    })
}

/// Row `row`, its cells read from the elements into `into`.
pub(crate) fn tree_row(tree: &TreeRead<'_>, row: RowIndex, into: &mut Vec<Value>) {
    into.clear();
    into.extend((0..tree.kinds.len()).map_while(|column| {
        tree_cell(
            tree,
            row,
            ColumnIndex(u32::try_from(column).unwrap_or(u32::MAX)),
        )
    }));
}

/// The rows of the tree read, copied into a table: for a node that cannot read the elements in place.
pub(crate) fn copied(tree: &TreeRead<'_>) -> Table {
    tree_table(tree.form, tree.kinds, tree.elements, tree.candidates)
}
