//! The `tree_walk` service: walking the tree of a file from the start rows of an `axis`, with no IO and no ports.

use ryo_language::domain::elements::service::{
    children, kind_names, next_sibling, previous_sibling,
};
use ryo_language::domain::relations::service::{node_file, node_id, node_position};
use ryo_language::domain::relations::types::{CellKind, Position, Value};

use super::constants::{NONE, SLOT_OF_ELEMENT, UNKNOWN};
use crate::domain::kind_sets::service::kind_tests;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::Remembering;
use crate::domain::operating::types::{
    ElementCount, Emit, Memo, OperatorContext, StartCount, WalkColumns, WalkShape,
};
use crate::domain::table_cells::types::{TreeForm, TreeRows};
use crate::domain::table_model::errors::TableModelError;
use crate::domain::table_model::service::{cell, column_in, count_of, positions, read_row, reset};
use crate::domain::table_model::types::{Rows, Table, Verdict};
use crate::domain::table_scratch::types::{ElementPlaces, KindsBy, Marks};
use crate::domain::text_gate::service::{needed_of_rest, text_gate, undecided};
use crate::domain::tree_join::service::tree_value;
use crate::domain::tree_walk::types::TreeWalk;
use crate::domain::walks::service::{admits, filter_of, give_filter};
use crate::domain::walks::types::Admission;
use ryo_language::domain::elements::service::{parent_at, size_at};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Direction, JoinColumn, Side};
use ryo_values::domain::plans::types::Operation;

/// How many elements the tree has.
fn elements_len(tree: &TreeRows<'_>) -> ElementCount {
    ElementCount(tree.elements.kind.len())
}

/// Where each entity of the tree is, among the entities the domain lists: marked by place.
fn mark(
    tree: &TreeRows<'_>,
    domain: &Rows<'_>,
    domain_id: ColumnIndex,
    row: &mut Vec<Value>,
    marks: &mut Marks,
) -> Result<(), OperatingError> {
    let marks = &mut marks.0;
    marks.clear();
    marks.resize(tree.elements.kind.len(), false);
    for position in positions(count_of(domain)) {
        let Some(admitted) = read_row(domain, position, row) else {
            continue;
        };
        if let Value::Node(entity) = cell(admitted, domain_id)?
            && node_file(entity) == tree.file
            && let Some(mark) =
                marks.get_mut(usize::try_from(node_position(entity).0).unwrap_or(usize::MAX))
        {
            *mark = true;
        }
    }
    Ok(())
}

/// Takes what the walk needs from the scratch of `context`, and makes ready what its domain decides when that is the elements
/// of a tree. `starts` is how many starts there will be, when known: many that each ask for the nearest ancestor are answered
/// from a table of the answers.
pub(crate) fn take_walk<'x, 'o>(
    tree: &TreeRows<'x>,
    domain: &Rows<'x>,
    direction: Direction,
    columns: WalkColumns,
    shape: WalkShape<'o>,
    starts: Option<StartCount>,
    context: &mut OperatorContext<'_>,
) -> TreeWalk<'x, 'o> {
    let WalkColumns { id, .. } = columns;
    let WalkShape {
        comparisons,
        output,
    } = shape;
    let marks = std::mem::take(&mut context.scratch.marks);
    let among = if let Rows::Tree(
        each @ TreeRows {
            form: TreeForm::Elements,
            ..
        },
    ) = domain
    {
        Some(*each)
    } else {
        None
    };
    let mut filter = filter_of(comparisons, context);
    if let Some(domain_tree) = &among {
        context.memo.clear();
        context
            .memo
            .resize(comparisons.len() * filter.names.0, Memo::Unknown);
        kind_tests(domain_tree, comparisons, context, &mut filter.kinds);
        text_gate(domain_tree, comparisons, &mut filter.gate);
        filter.text_ok.0.clear();
        if !filter.gate.0.is_empty() {
            filter.text_ok.0.resize(filter.names.0 + 1, 0);
        }
        let kinds_by = if kind_names(domain_tree.elements).is_empty() {
            KindsBy::Nothing
        } else {
            KindsBy::Sets
        };
        undecided(comparisons, &filter.gate, kinds_by, &mut filter.rest);
        needed_of_rest(comparisons, &filter.rest, &mut filter.needed);
    }
    let mut nearest = std::mem::take(&mut context.scratch.nearest);
    let remembering = if direction == Direction::NearestAncestor
        && starts.is_none_or(|count| count.0.saturating_mul(8) > elements_len(tree).0)
    {
        Remembering::Kept
    } else {
        Remembering::Forgotten
    };
    if remembering == Remembering::Kept {
        nearest.0.clear();
        nearest.0.resize(elements_len(tree).0, UNKNOWN);
    }
    TreeWalk {
        tree: *tree,
        among,
        direction,
        id,
        output,
        remembering,
        marks,
        found: std::mem::take(&mut context.scratch.candidates),
        nearest,
        climbing: std::mem::take(&mut context.scratch.climbing),
        filter,
    }
}

/// Marks, when the domain of the walk is a table, where the entities it lists are in the tree.
///
/// # Errors
///
/// The domain has no such column, or a row of it is wrong.
pub(crate) fn mark_domain(
    walk: &mut TreeWalk<'_, '_>,
    domain: &Rows<'_>,
    domain_id: ColumnIndex,
    context: &mut OperatorContext<'_>,
) -> Result<(), OperatingError> {
    if walk.among.is_some() {
        return Ok(());
    }
    let mut row = std::mem::take(&mut context.scratch.row);
    let marked = mark(&walk.tree, domain, domain_id, &mut row, &mut walk.marks);
    context.scratch.row = row;
    marked
}

/// Gives back what the walk took.
pub(crate) fn give_walk(walk: TreeWalk<'_, '_>, context: &mut OperatorContext<'_>) {
    context.scratch.marks = walk.marks;
    context.scratch.candidates = walk.found;
    context.scratch.nearest = walk.nearest;
    context.scratch.climbing = walk.climbing;
    give_filter(walk.filter, context);
}

/// Whether the element at `found` is an answer.
fn admitted(
    walk: &mut TreeWalk<'_, '_>,
    reached: &TreeRows<'_>,
    found: Position,
    context: &mut OperatorContext<'_>,
) -> Result<Verdict, OperatingError> {
    let admission = if walk.among.is_some() {
        Admission::Filtered(walk.filter.comparisons)
    } else {
        Admission::Marked(&walk.marks)
    };
    admits(&admission, &mut walk.filter, reached, found, context)
}

/// The remembered nearest ancestor of `element`, an element, [`UNKNOWN`] or [`NONE`]; crashes when `element` has no slot.
#[expect(
    clippy::expect_used,
    reason = "the remembered answers hold one slot per element of the tree; an element without one is a bug that must crash, not a walk that finds nothing"
)]
pub(crate) fn nearest_at(nearest: &ElementPlaces, element: Position) -> Position {
    nearest
        .0
        .get(usize::try_from(element.0).unwrap_or(usize::MAX))
        .map(|known| Position(*known))
        .expect(SLOT_OF_ELEMENT)
}

/// The nearest admitted ancestor of the element at `at`, asked once for each element and answered from its parent's.
#[expect(
    clippy::indexing_slicing,
    reason = "each element written was read through `nearest_at` first, which crashes when its slot is missing"
)]
fn nearest_of(
    walk: &mut TreeWalk<'_, '_>,
    reached: &TreeRows<'_>,
    at: Position,
    context: &mut OperatorContext<'_>,
) -> Result<(), OperatingError> {
    let elements = walk.tree.elements;
    walk.climbing.0.clear();
    // The place of an element among the remembered answers.
    let place = |element: Position| usize::try_from(element.0).unwrap_or(usize::MAX);
    let mut here = at;
    while nearest_at(&walk.nearest, here).0 == UNKNOWN {
        if here.0 == 0 {
            walk.nearest.0[place(here)] = NONE;
            break;
        }
        walk.climbing.0.push(here.0);
        here = parent_at(elements, here);
    }
    while let Some(each) = walk.climbing.0.pop() {
        let parent = parent_at(elements, Position(each));
        let verdict = admitted(walk, reached, parent, context)?;
        let nearest = match verdict {
            Verdict::Holds => parent.0,
            Verdict::Fails => nearest_at(&walk.nearest, parent).0,
        };
        walk.nearest.0[place(Position(each))] = nearest;
    }
    let found = nearest_at(&walk.nearest, at).0;
    if found != NONE {
        walk.found.0.push(found);
    }
    Ok(())
}

/// Passes on the rows that the start `from_row` reaches, each the start with the entity found, or the columns the output names.
///
/// # Errors
///
/// A column the row lacks, or what a comparison or the one that takes the rows refuses.
pub(crate) fn walk_row(
    walk: &mut TreeWalk<'_, '_>,
    from_row: &[Value],
    context: &mut OperatorContext<'_>,
    emit: &mut dyn Emit,
    answer: &mut Vec<Value>,
) -> Result<(), OperatingError> {
    let tree = walk.tree;
    let Value::Node(start) = cell(from_row, walk.id)? else {
        return Ok(());
    };
    let at = node_position(start);
    let elements = tree.elements;
    let count = elements.kind.len();
    if node_file(start) != tree.file || usize::try_from(at.0).map_or(true, |place| place >= count) {
        return Ok(());
    }
    let reached = walk.among.unwrap_or(tree);
    let direction = walk.direction;
    let up = |from: Position| (from.0 != 0).then(|| parent_at(elements, from));
    walk.found.0.clear();
    if direction == Direction::NearestAncestor && walk.remembering == Remembering::Kept {
        nearest_of(walk, &reached, at, context)?;
    } else {
        match direction {
            Direction::Parent => walk.found.0.extend(up(at).map(|each| each.0)),
            Direction::Child => walk
                .found
                .0
                .extend(children(elements, at).map(|each| each.0)),
            Direction::Ancestor | Direction::NearestAncestor => walk
                .found
                .0
                .extend(std::iter::successors(up(at), |each| up(*each)).map(|each| each.0)),
            Direction::Descendant => walk
                .found
                .0
                .extend(at.0 + 1..=at.0 + size_at(elements, at).0),
            // Below the children: the elements whose parent is not the start.
            Direction::Deep => walk.found.0.extend(
                (at.0 + 1..=at.0 + size_at(elements, at).0)
                    .filter(|each| parent_at(elements, Position(*each)) != at),
            ),
            Direction::PreviousSibling => walk
                .found
                .0
                .extend(previous_sibling(elements, at).map(|each| each.0)),
            Direction::NextSibling => walk
                .found
                .0
                .extend(next_sibling(elements, at).map(|each| each.0)),
        }
    }
    for place in 0..walk.found.0.len() {
        let Some(&each) = walk.found.0.get(place) else {
            break;
        };
        if admitted(walk, &reached, Position(each), context)? == Verdict::Fails {
            continue;
        }
        answer.clear();
        if walk.output.is_empty() {
            answer.extend_from_slice(from_row);
            answer.push(Value::Node(node_id(tree.file, Position(each))));
        } else {
            for column in walk.output {
                answer.push(match column.side {
                    Side::Left => cell(from_row, column.column)?,
                    Side::Right => tree_value(&reached, Position(each), column.column),
                });
            }
        }
        emit.row(context, answer)?;
        if direction == Direction::NearestAncestor {
            break;
        }
    }
    Ok(())
}

/// The columns of the rows a walk from `from` makes.
///
/// # Errors
///
/// An output column that the side it names does not have.
pub(crate) fn walk_kinds(
    from: &[CellKind],
    reached: &[CellKind],
    output: &[JoinColumn],
    out: &mut Table,
) -> Result<(), OperatingError> {
    reset(out, &[]);
    if output.is_empty() {
        out.kinds.extend_from_slice(from);
        out.kinds.push(CellKind::Node);
        return Ok(());
    }
    for column in output {
        let kinds = match column.side {
            Side::Left => from,
            Side::Right => reached,
        };
        out.kinds.push(
            *column_in(kinds, column.column).ok_or(TableModelError::NoSuchColumn {
                index: column.column.0,
            })?,
        );
    }
    Ok(())
}

/// The column of the domain that holds the entity a walk reaches: the domain's id, through its view (a column the view does not name is the domain's own).
pub(crate) fn entity_of(domain_view: &[ColumnIndex], domain_id: ColumnIndex) -> ColumnIndex {
    match domain_view.get(usize::try_from(domain_id.0).unwrap_or(usize::MAX)) {
        Some(moved) => *moved,
        None => domain_id,
    }
}

/// Starts the walk of an axis over `tree`: its room taken from the scratch, and the elements of its domain marked.
///
/// # Errors
///
/// `WrongOperation` when the operation is not an axis, or what marking the domain refuses.
pub(crate) fn start_walk<'x, 'o>(
    operation: &'o Operation,
    tree: &TreeRows<'x>,
    domain: &Rows<'x>,
    starts: Option<StartCount>,
    context: &mut OperatorContext<'_>,
) -> Result<TreeWalk<'x, 'o>, OperatingError> {
    let Operation::Axis {
        direction,
        id,
        domain_id,
        domain_filter,
        domain_view,
        output,
        ..
    } = operation
    else {
        return Err(OperatingError::WrongOperation);
    };
    let entity = entity_of(domain_view, *domain_id);
    let mut walk = take_walk(
        tree,
        domain,
        *direction,
        WalkColumns {
            id: *id,
            domain: entity,
        },
        WalkShape {
            comparisons: domain_filter,
            output,
        },
        starts,
        context,
    );
    mark_domain(&mut walk, domain, entity, context)?;
    Ok(walk)
}
