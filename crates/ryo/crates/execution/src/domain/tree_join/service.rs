//! The `tree_join` service: how a join finds the elements of a tree for each row of its left input, with no IO and no ports.

use ryo_language::domain::elements::service::{field_name_at, kind_names};
use ryo_language::domain::symbols::service::name_count;
use ryo_relations::domain::relations::service::node_id;
use ryo_relations::domain::relations::types::{Flag, Position, Value};

use super::types::{ColumnLack, TreeState};
use crate::domain::comparing::service::holds_some;
use crate::domain::kind_sets::service::{candidates_of, kind_tests, passes};
use crate::domain::kind_sets::types::KindScratch;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::{Emit, Memo, NameCount, OperatorContext, TreeShape};
use crate::domain::operating::types::{KindNarrowing, MissingColumn};
use crate::domain::operator::contract::types::{PrimitiveSemantics, join_result};
use crate::domain::table_cells::service::{
    kind_name_at, ordinal_number_at, read_tree_cells, span_at,
};
use crate::domain::table_cells::types::{CellsRead, TreeForm, TreeRows};
use crate::domain::table_model::service::{count_of, kinds_of, read_row};
use crate::domain::table_model::types::{Rows, Verdict};
use crate::domain::table_scratch::types::{ElementIndex, KindsBy};
use crate::domain::text_gate::service::{
    needed_of, needed_of_rest, text_admitted, text_gate, undecided,
};
use crate::domain::tree_join::collections::strategies::entity_vector::item::EntityVectorItem;
use crate::domain::tree_join::collections::strategies::interface::StrategiesItem as _;
use crate::domain::tree_join::collections::strategies::span_index::item::SpanIndexItem;
use crate::domain::tree_join::collections::strategies::types::{Fitting, NodeColumns};
use ryo_derived::domain::contract::types::Finding;
use ryo_language::domain::elements::service::{kind_at, parent_at, text_at};
use ryo_operators::inbound::use_cases::operators::interface::OperatorCall;
use ryo_values::domain::plan_parts::types::{ColumnIndex, PlanComparison};
use ryo_values::domain::rows::errors::TableModelError;
mod driver;

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

/// Takes what the join needs from the scratch of `context`, and makes ready what its comparisons decide.
pub(crate) fn take_state(
    tree: &TreeRows<'_>,
    comparisons: &[PlanComparison],
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
        key: std::mem::take(&mut context.scratch.join_scratch.key),
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
    context.scratch.join_scratch.key = state.key;
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
    semantics: &dyn PrimitiveSemantics,
    left_on: &[ColumnIndex],
    work: &crate::domain::operator::contract::types::WorkCounters,
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
    let mut target = std::mem::take(&mut state.other);
    let mut key = std::mem::take(&mut state.key);
    #[cfg(feature = "diagnostic-row-progress")]
    (semantics.progress())(
        crate::domain::operator::contract::views::ProgressCounter::InputRows,
        1,
    );
    let mut driver = driver::Driver {
        #[cfg(feature = "diagnostic-row-progress")]
        progress: semantics.progress(),
        state,
        work,
        tree,
        comparisons,
        context,
        emit,
        at: 0,
        current: None,
        error: None,
    };
    let answer = semantics.operators(OperatorCall::EqualityJoin {
        input: ryo_operators::inbound::use_cases::operators::interface::JoinInput::Values {
            left: ryo_operators::inbound::use_cases::operators::interface::JoinRow::Values(
                left_row,
            ),
            left_on,
            output,
            view,
            driver: &mut driver,
            key: &mut key,
            target: &mut target,
        },
    });
    let error = driver.error.take();
    driver.state.other = target;
    driver.state.key = key;
    join_result(answer, error)
}

/// Joins the rows of `left` with the rows of a tree that a filter admits, found by the key: an element
/// by its id, an edge by its child or by its parent, read where the tree says and only if the filter admits it.
/// No table of them is made.
///
/// # Errors
///
/// Whatever the probe of a row or the one that takes the rows refuse.
pub(crate) fn join_tree(
    semantics: &dyn PrimitiveSemantics,
    left_on: &[ColumnIndex],
    tree: &TreeRows<'_>,
    left: &Rows<'_>,
    work: &crate::domain::operator::contract::types::WorkCounters,
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
        if narrowed {
            candidates.0.sort_unstable();
        }
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
            work.found(0, 1);
            outcome = probe(
                semantics, left_on, work, &mut state, tree, finding, shape, left_row, context, emit,
            );
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

use ryo_values::domain::rows::types::RowIndex;
