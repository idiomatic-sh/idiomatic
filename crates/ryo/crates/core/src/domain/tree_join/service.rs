//! The `tree_join` service: how a join finds the elements of a tree for each row of its left input, with no IO and no ports.

use ryo_language::domain::elements::service::{field_name_at, kind_names};
use ryo_relations::domain::relations::service::node_id;
use ryo_relations::domain::relations::types::{Flag, Position, Value};
use ryo_language::domain::symbols::service::name_count;

use super::types::{ColumnLack, TreeState};
use crate::domain::comparing::service::holds_some;
use crate::domain::kind_sets::service::{candidates_of, kind_tests, passes};
use crate::domain::kind_sets::types::KindScratch;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::{Emit, Memo, NameCount, OperatorContext, TreeShape};
use crate::domain::operating::types::{KindNarrowing, MissingColumn};
use crate::domain::table_cells::service::{
    kind_name_at, ordinal_number_at, read_tree_cells, span_at,
};
use crate::domain::table_cells::types::{CellsRead, RowIndex, TreeForm, TreeRows};
use crate::domain::table_model::errors::TableModelError;
use crate::domain::table_model::service::{cell, count_of, kinds_of, read_row};
use crate::domain::table_model::types::{Rows, Verdict};
use crate::domain::table_scratch::types::{ElementIndex, KindsBy};
use crate::domain::text_gate::service::{
    needed_of, needed_of_rest, text_admitted, text_gate, undecided,
};
use crate::domain::tree_join::collections::strategies::entity_vector::item::EntityVectorItem;
use crate::domain::tree_join::collections::strategies::interface::StrategiesItem as _;
use crate::domain::tree_join::collections::strategies::span_index::item::SpanIndexItem;
use crate::domain::tree_join::collections::strategies::types::{Finding, Fitting, NodeColumns};
use ryo_language::domain::elements::service::{kind_at, parent_at, text_at};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Comparison, Side};

/// The column of the elements of a tree that the column `column` of the view of them is.
pub(crate) fn source_column(view: &[ColumnIndex], column: ColumnIndex) -> ColumnIndex {
    match view.get(usize::try_from(column.0).unwrap_or(usize::MAX)) {
        Some(moved) => *moved,
        // A view names only the columns it moves: a column it does not name is the tree's own.
        None => column,
    }
}

/// One cell of the row of the tree at `at`: of the edge to the element, or of the element.
pub(crate) fn tree_value(tree: &TreeRows<'_>, at: Position, column: ColumnIndex) -> Value {
    let elements = tree.elements;
    if tree.form == TreeForm::Edges {
        return match column.0 {
            0 => Value::Node(node_id(tree.file, parent_at(elements, at))),
            1 => Value::Node(node_id(tree.file, at)),
            2 => Value::Number(ordinal_number_at(elements, at)),
            3 => Value::Symbol(field_name_at(elements, at)),
            4 => Value::File(tree.file),
            _ => Value::Flag(Flag::Lowered),
        };
    }
    match column.0 {
        0 => Value::Node(node_id(tree.file, at)),
        1 => Value::Symbol(kind_name_at(elements, at)),
        2 => Value::Symbol(text_at(elements, at)),
        3 => Value::File(tree.file),
        4 => Value::Span(span_at(elements, at)),
        _ => Value::Flag(Flag::Lowered),
    }
}

/// What finds the elements of the tree for a row of the left input, when the join asks for them by what finds them.
pub(crate) fn tree_finding(
    tree: &TreeRows<'_>,
    left_on: &[ColumnIndex],
    right_on: &[ColumnIndex],
    view: &[ColumnIndex],
) -> Option<Finding> {
    if let ([left_key], [right_key]) = (left_on, right_on) {
        let key = source_column(view, *right_key);
        // The elements that cover the stretch the left row holds.
        if key == ColumnIndex(4) && tree.form == TreeForm::Elements {
            return Some(Finding::Span(*left_key));
        }
        if key == ColumnIndex(0) || key == ColumnIndex(1) && tree.form == TreeForm::Edges {
            return Some(Finding::Node(*left_key, key));
        }
    }
    if let ([left_file, left_stretch], [right_file, right_stretch]) = (left_on, right_on)
        && tree.form == TreeForm::Elements
    {
        return match (
            source_column(view, *right_file),
            source_column(view, *right_stretch),
        ) {
            (ColumnIndex(3), ColumnIndex(4)) => Some(Finding::Stretch(*left_file, *left_stretch)),
            (ColumnIndex(4), ColumnIndex(3)) => Some(Finding::Stretch(*left_stretch, *left_file)),
            _ => None,
        };
    }
    None
}

/// Takes what the join needs from the scratch of `context`, and makes ready what its comparisons decide.
pub(crate) fn take_state(
    tree: &TreeRows<'_>,
    comparisons: &[Comparison],
    context: &mut OperatorContext<'_>,
) -> TreeState {
    let names = NameCount(name_count(context.symbols.as_ref()).0);
    context.memo.clear();
    context
        .memo
        .resize(comparisons.len() * names.0, Memo::Unknown);
    let mut other = std::mem::take(&mut context.scratch.other);
    other.clear();
    let mut needed = std::mem::take(&mut context.scratch.needed);
    let mut kinds = KindScratch {
        names: std::mem::take(&mut context.scratch.select_scratch.kind_names),
        sets: std::mem::take(&mut context.scratch.select_scratch.kind_sets),
        tests: std::mem::take(&mut context.scratch.select_scratch.kind_tests),
    };
    needed_of(comparisons, &mut needed);
    kind_tests(tree, comparisons, context, &mut kinds);
    let tree_rows = Rows::Tree(*tree);
    let elements = kinds_of(&tree_rows);
    let missing = needed
        .0
        .iter()
        .find(|place| **place >= elements.len())
        .copied()
        .map_or(ColumnLack::Nothing, |place| {
            ColumnLack::Column(MissingColumn(place))
        });
    // What the kind sets decided for an element is not asked of it again.
    let kinds_by = if tree.form == TreeForm::Elements && !kind_names(tree.elements).is_empty() {
        KindsBy::Sets
    } else {
        KindsBy::Nothing
    };
    let mut gate = std::mem::take(&mut context.scratch.select_scratch.gate);
    let mut text_ok = std::mem::take(&mut context.scratch.select_scratch.text_ok);
    text_gate(tree, comparisons, &mut gate);
    text_ok.0.clear();
    if !gate.0.is_empty() {
        text_ok.0.resize(names.0 + 1, 0);
    }
    let mut rest = std::mem::take(&mut context.scratch.select_scratch.rest);
    undecided(comparisons, &gate, kinds_by, &mut rest);
    needed_of_rest(comparisons, &rest, &mut needed);
    TreeState {
        names,
        other,
        needed,
        kinds,
        gate,
        text_ok,
        rest,
        stretched: Vec::new(),
        candidates: std::mem::take(&mut context.scratch.select_scratch.candidates),
        cells: [Value::Flag(Flag::Lowered); 5],
        missing,
    }
}

/// Gives back what was taken.
pub(crate) fn give_back(state: TreeState, context: &mut OperatorContext<'_>) {
    context.scratch.select_scratch.gate = state.gate;
    context.scratch.select_scratch.text_ok = state.text_ok;
    context.scratch.other = state.other;
    context.scratch.needed = state.needed;
    context.scratch.select_scratch.rest = state.rest;
    context.scratch.select_scratch.candidates = state.candidates;
    context.scratch.select_scratch.kind_names = state.kinds.names;
    context.scratch.select_scratch.kind_sets = state.kinds.sets;
    context.scratch.select_scratch.kind_tests = state.kinds.tests;
}

/// Passes on the rows that the row `left_row` of the left input finds in the tree, and the filter admits.
///
/// # Errors
///
/// `NoSuchColumn` when a comparison reads a column the elements lack or the row lacks one; whatever the one that takes the rows
/// refuses.
pub(crate) fn probe(
    state: &mut TreeState,
    tree: &TreeRows<'_>,
    finding: Finding,
    shape: TreeShape<'_>,
    left_row: &[Value],
    context: &mut OperatorContext<'_>,
    emit: &mut dyn Emit,
) -> Result<(), OperatingError> {
    let TreeShape {
        comparisons,
        view,
        output,
    } = shape;
    if let ColumnLack::Column(missing) = state.missing {
        return Err(OperatingError::Table(TableModelError::NoSuchColumn {
            index: u32::try_from(missing.0).unwrap_or(u32::MAX),
        }));
    }
    let first = usize::from(tree.form == TreeForm::Edges);
    let tree_rows = Rows::Tree(*tree);
    let elements = kinds_of(&tree_rows);
    state.stretched.clear();
    match finding {
        Finding::Node(left_key, key)
            if EntityVectorItem.applies(Some(finding)) == Fitting::Fits =>
        {
            EntityVectorItem::find(
                tree,
                left_row,
                NodeColumns {
                    left: left_key,
                    key,
                },
                &mut state.stretched,
            )?;
        }
        _ if SpanIndexItem.applies(Some(finding)) == Fitting::Fits => {
            SpanIndexItem::find(tree, left_row, finding, &mut state.stretched)?;
        }
        Finding::Node(..) | Finding::Stretch(..) | Finding::Span(_) => {}
    }
    for place in 0..state.stretched.len() {
        let Some(&found) = state.stretched.get(place) else {
            break;
        };
        if !state.kinds.tests.0.is_empty()
            && passes(&state.kinds, kind_at(tree.elements, found)) == Verdict::Fails
        {
            continue;
        }
        let index = RowIndex(usize::try_from(found.0).unwrap_or(usize::MAX) - first);
        if !state.gate.0.is_empty()
            && text_admitted(
                tree,
                ElementIndex(usize::try_from(found.0).unwrap_or(usize::MAX)),
                comparisons,
                &state.gate,
                &mut state.text_ok,
                state.names,
                &mut state.cells,
                elements,
                context,
            )? == Verdict::Fails
        {
            continue;
        }
        if !state.rest.0.is_empty()
            && (read_tree_cells(tree, index, &mut state.cells, &state.needed) == CellsRead::Missing
                || holds_some(
                    comparisons,
                    &state.rest,
                    &state.cells,
                    elements,
                    state.names,
                    context,
                )? == Verdict::Fails)
        {
            continue;
        }
        state.other.clear();
        for column in output {
            state.other.push(match column.side {
                Side::Left => cell(left_row, column.column)?,
                Side::Right => tree_value(tree, found, source_column(view, column.column)),
            });
        }
        emit.row(context, &state.other)?;
    }
    Ok(())
}

/// Joins the rows of `left` with the rows of a tree that a filter admits, found by the key: an element
/// by its id, an edge by its child or by its parent, read where the tree says and only if the filter admits it.
/// No table of them is made.
///
/// # Errors
///
/// Whatever the probe of a row or the one that takes the rows refuse.
pub(crate) fn join_tree(
    tree: &TreeRows<'_>,
    left: &Rows<'_>,
    finding: Finding,
    shape: TreeShape<'_>,
    context: &mut OperatorContext<'_>,
    emit: &mut dyn Emit,
) -> Result<(), OperatingError> {
    let comparisons = shape.comparisons;
    // No row on the left finds nothing in the tree.
    if count_of(left).0 == 0 {
        return Ok(());
    }
    let mut state = take_state(tree, comparisons, context);
    let mut buffer = std::mem::take(&mut context.scratch.row);
    let outcome = {
        // Each edge of the tree that asks for its child, and a filter that names few kinds: only the edges of the children of
        // those kinds can be joined, and the elements of a kind are found without a pass over the tree.
        let by_child = matches!(
            left,
            Rows::Tree(TreeRows {
                form: TreeForm::Edges,
                ..
            })
        ) && matches!(finding, Finding::Node(ColumnIndex(1), ColumnIndex(0)))
            && tree.form == TreeForm::Elements;
        let mut candidates = std::mem::take(&mut state.candidates);
        let narrowed = by_child
            && candidates_of(tree, &state.kinds, &mut candidates) == KindNarrowing::Narrowed;
        let places = if narrowed {
            candidates.0.len()
        } else {
            count_of(left).0
        };
        let mut outcome = Ok(());
        for place in 0..places {
            let position = if narrowed {
                // The edge of the element at a place is the row before it: the root has none.
                match candidates.0.get(place).and_then(|candidate| {
                    usize::try_from(*candidate)
                        .unwrap_or(usize::MAX)
                        .checked_sub(1)
                }) {
                    Some(row) => RowIndex(row),
                    None => continue,
                }
            } else {
                RowIndex(place)
            };
            let Some(left_row) = read_row(left, position, &mut buffer) else {
                continue;
            };
            outcome = probe(&mut state, tree, finding, shape, left_row, context, emit);
            if outcome.is_err() {
                break;
            }
        }
        state.candidates = candidates;
        outcome
    };
    context.scratch.row = buffer;
    give_back(state, context);
    outcome
}
