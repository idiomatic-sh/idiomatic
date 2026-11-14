//! The `scan_roots` item of the `kernels` collection: a tree relation's rows read in place from the open file's elements, at
//! the tree nodes the engine names for the reading node, in tree order; one cell at a time, never decoded into columns.

#[cfg(test)]
use super::super::interface::{KernelsItem, Reach};
use crate::domain::physical_plan::types::{PlanInputs, PlanNodeId, TreeRead};
#[cfg(test)]
use crate::domain::physical_plan::types::{RowSource, lowering::value::Lowering};
use crate::domain::tree_cells::service::{native_form, tree_value};
use crate::domain::tree_cells::types::{TreeForm, TreeRows};
use ryo_language::domain::elements::types::Elements;
use ryo_relations::domain::relations::service::{kinds, node_file, node_position};
use ryo_relations::domain::relations::types::{CellKind, Relation, Value};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::{RowCount, RowIndex};

/// `scan_roots`, one item of the `kernels` collection.
#[cfg(test)]
pub(crate) struct ScanRootsItem;

#[cfg(test)]
impl KernelsItem for ScanRootsItem {
    fn reached_by(&self, lowering: &Lowering) -> Reach {
        let runs = lowering
            .sources()
            .any(|source| matches!(source, RowSource::Tree(_)));
        if runs { Reach::Runs } else { Reach::Skips }
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
    let form = native_form(relation).unwrap_or(TreeForm::Elements);
    let candidates = inputs.roots(node).0;
    let candidates = match (form, candidates.split_first()) {
        (TreeForm::Edges, Some((first, rest))) if node_position(*first).0 == 0 => rest,
        (TreeForm::Edges | TreeForm::Elements, _) => candidates,
    };
    TreeRead {
        form,
        kinds: kinds(relation),
        elements,
        candidates,
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
