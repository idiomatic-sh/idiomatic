//! The `walking` service: free functions named by verb over the descents and neighbour lookups of the syntax tree made one walk.

use ryo_language::domain::relations::service::columns;
use ryo_language::domain::relations::types::Relation;

use super::types::{ElementDemand, InputWidth, StartingRow};
use crate::domain::fusing::service::source_column;
use crate::domain::rewriting::types::StepIndex;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Comparison;
use ryo_values::domain::plan_parts::types::Direction as AxisDirection;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plan_parts::types::JoinColumn;
use ryo_values::domain::plan_parts::types::Operand;
use ryo_values::domain::plan_parts::types::QueryLiteral;
use ryo_values::domain::plan_parts::types::Side;
use ryo_values::domain::plan_parts::types::Test;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::plans::types::PlanStep;

/// How many columns the rows of `input` have, when the plan says so.
pub(crate) fn width_of(steps: &[PlanStep], input: Input) -> Option<InputWidth> {
    match input {
        Input::Relation(relation) => Some(InputWidth(
            u32::try_from(columns(relation).len()).unwrap_or(u32::MAX),
        )),
        Input::Step(id) => match &steps
            .get(usize::try_from(id.0).unwrap_or(usize::MAX))?
            .operation
        {
            Operation::Select { from, kept, .. } => {
                if kept.is_empty() {
                    width_of(steps, *from)
                } else {
                    Some(InputWidth(u32::try_from(kept.len()).unwrap_or(u32::MAX)))
                }
            }
            Operation::Join { output, .. } => {
                Some(InputWidth(u32::try_from(output.len()).unwrap_or(u32::MAX)))
            }
            Operation::Project { columns, .. } => {
                Some(InputWidth(u32::try_from(columns.len()).unwrap_or(u32::MAX)))
            }
            Operation::Axis { from, output, .. } => {
                if output.is_empty() {
                    width_of(steps, *from).map(|width| InputWidth(width.0 + 1))
                } else {
                    Some(InputWidth(u32::try_from(output.len()).unwrap_or(u32::MAX)))
                }
            }
            Operation::Extend { .. }
            | Operation::Union { .. }
            | Operation::Intersection { .. }
            | Operation::Difference { .. }
            | Operation::Antijoin { .. }
            | Operation::GroupCount { .. }
            | Operation::Replace { .. }
            | Operation::SpanBounds { .. }
            | Operation::SourceMatches { .. }
            | Operation::RenderSyntax { .. }
            | Operation::FitsRendered { .. }
            | Operation::SpanBetween { .. }
            | Operation::OrderedMatch { .. }
            | Operation::OrderedDfs { .. }
            | Operation::GroupDigest { .. }
            | Operation::Capture { .. }
            | Operation::Rank { .. }
            | Operation::Words { .. }
            | Operation::Closure { .. }
            | Operation::Cycles { .. }
            | Operation::Rows { .. }
            | Operation::Hosted { .. } => None,
        },
    }
}

/// Whether the comparisons ask only that an element of the syntax tree be a node: a node has no text.
fn only_nodes(comparisons: &[Comparison]) -> ElementDemand {
    if matches!(
        comparisons,
        [Comparison {
            test: Test::Equal,
            left: Operand::Column(ColumnIndex(2)),
            right: Operand::Literal(QueryLiteral::Text(text)),
            ..
        }] if text.0.is_empty()
    ) {
        ElementDemand::OnlyNodes
    } else {
        ElementDemand::More
    }
}

/// The columns the fused descent gives: those the second join kept, found in the elements, or in the starting row of `width` columns.
fn descent_columns(
    output: &[JoinColumn],
    view: &[ColumnIndex],
    edges_output: &[JoinColumn],
    width: InputWidth,
) -> Option<Vec<JoinColumn>> {
    let mut columns = Vec::new();
    for kept in output {
        columns.push(match kept.side {
            Side::Right => JoinColumn {
                side: Side::Right,
                column: source_column(view, kept.column)?,
            },
            Side::Left => {
                let was = edges_output.get(usize::try_from(kept.column.0).unwrap_or(usize::MAX))?;
                match (was.side, was.column) {
                    (Side::Right, ColumnIndex(1)) => JoinColumn {
                        side: Side::Right,
                        column: ColumnIndex(0),
                    },
                    (Side::Left, column) if column.0 < width.0 => JoinColumn {
                        side: Side::Left,
                        column,
                    },
                    _ => return None,
                }
            }
        });
    }
    Some(columns)
}

/// The join at `place` that finds, among the children of every descendant of a starting node, the elements a filter admits,
/// made one walk over the elements below the starting node's children: no table of the descendants, nor of their children, is made.
fn fuse_descent(steps: &[PlanStep], place: StepIndex) -> Option<Operation> {
    let Operation::Join {
        left: Input::Step(children),
        right: Input::Relation(Relation::SyntaxTree),
        left_on,
        right_on,
        output,
        filter,
        view,

        reach: None,
    } = &steps.get(place.0)?.operation
    else {
        return None;
    };
    let ([left_key], [right_key]) = (left_on.as_slice(), right_on.as_slice()) else {
        return None;
    };
    if source_column(view, *right_key)? != ColumnIndex(0) {
        return None;
    }
    let middle = steps.get(usize::try_from(children.0).unwrap_or(usize::MAX))?;
    let Operation::Join {
        left: Input::Step(walked),
        right: Input::Relation(Relation::SyntaxChild),
        left_on: through,
        right_on: by_parent,
        output: edges_output,
        filter: no_filter,
        view: no_view,

        reach: None,
    } = &middle.operation
    else {
        return None;
    };
    let first_axis = steps.get(usize::try_from(walked.0).unwrap_or(usize::MAX))?;
    let Operation::Axis {
        from,
        edges: Input::Relation(Relation::SyntaxChild),
        domain: Input::Relation(Relation::SyntaxTree),
        direction: AxisDirection::Descendant,
        id,
        parent: ColumnIndex(0),
        child: ColumnIndex(1),
        domain_id,
        domain_filter,
        domain_view,
        output: no_output,
    } = &first_axis.operation
    else {
        return None;
    };
    let scope = steps.get(place.0)?.scope;
    if middle.scope != scope
        || first_axis.scope != scope
        || !no_filter.is_empty()
        || !no_view.is_empty()
        || !no_output.is_empty()
        || by_parent.as_slice() != [ColumnIndex(0)]
        || only_nodes(domain_filter) == ElementDemand::More
        || source_column(domain_view, *domain_id)? != ColumnIndex(0)
    {
        return None;
    }
    // The join reads the descendant the axis reached, which is the column after those of the starting row.
    let width = width_of(steps, *from)?;
    if through.as_slice() != [ColumnIndex(width.0)] {
        return None;
    }
    // The element the second join asks for is the child the first one found.
    let found = edges_output.get(usize::try_from(left_key.0).unwrap_or(usize::MAX))?;
    if (found.side, found.column) != (Side::Right, ColumnIndex(1)) {
        return None;
    }
    let columns = descent_columns(output, view, edges_output, width)?;
    Some(Operation::Axis {
        from: *from,
        edges: Input::Relation(Relation::SyntaxChild),
        domain: Input::Relation(Relation::SyntaxTree),
        direction: AxisDirection::Deep,
        id: *id,
        parent: ColumnIndex(0),
        child: ColumnIndex(1),
        domain_id: ColumnIndex(0),
        domain_filter: filter.clone(),
        domain_view: Vec::new(),
        output: columns,
    })
}

/// The steps with each descent whose answer is found in the children of what it reached made one walk.
#[must_use]
pub(crate) fn fuse_descents(mut steps: Vec<PlanStep>) -> Vec<PlanStep> {
    for place in 0..steps.len() {
        if let Some(fused) = fuse_descent(&steps, StepIndex(place))
            && let Some(step) = steps.get_mut(place)
        {
            step.operation = fused;
        }
    }
    steps
}

/// The columns the fused lookup gives, as the axis has them: those the second join kept, found in the elements the axis reaches, or
/// in the row it starts from, `width` columns wide, whose node is at `start`. The edge the first join made is not made: its parent
/// or child is the node the row holds or the element reached, and its file is the file of the element; its ordinal is not given.
fn neighbour_columns(
    output: &[JoinColumn],
    view: &[ColumnIndex],
    edges_output: &[JoinColumn],
    row: StartingRow,
) -> Option<Vec<JoinColumn>> {
    let StartingRow {
        width,
        start,
        found,
    } = row;
    let mut columns = Vec::new();
    for kept in output {
        columns.push(match kept.side {
            Side::Right => JoinColumn {
                side: Side::Right,
                column: source_column(view, kept.column)?,
            },
            Side::Left => {
                let was = edges_output.get(usize::try_from(kept.column.0).unwrap_or(usize::MAX))?;
                match (was.side, was.column) {
                    // The edge's end that is the element reached.
                    (Side::Right, column) if column == found => JoinColumn {
                        side: Side::Right,
                        column: ColumnIndex(0),
                    },
                    // The edge's other end, which is the node the row holds.
                    (Side::Right, ColumnIndex(0 | 1)) => JoinColumn {
                        side: Side::Left,
                        column: start,
                    },
                    (Side::Right, ColumnIndex(3)) => JoinColumn {
                        side: Side::Right,
                        column: ColumnIndex(3),
                    },
                    (Side::Left, column) if column.0 < width.0 => JoinColumn {
                        side: Side::Left,
                        column,
                    },
                    (Side::Left | Side::Right, _) => return None,
                }
            }
        });
    }
    Some(columns)
}

/// The join at `place` that finds, among the children or the parent of the node a row holds, the elements a filter admits, made
/// one axis: the edges of the syntax tree are not made into rows, and the elements are decided where they are.
fn fuse_neighbour(steps: &[PlanStep], place: StepIndex) -> Option<Operation> {
    let Operation::Join {
        left: Input::Step(edges),
        right: Input::Relation(Relation::SyntaxTree),
        left_on,
        right_on,
        output,
        filter,
        view,

        reach: None,
    } = &steps.get(place.0)?.operation
    else {
        return None;
    };
    let ([found_at], [element_key]) = (left_on.as_slice(), right_on.as_slice()) else {
        return None;
    };
    if source_column(view, *element_key)? != ColumnIndex(0) {
        return None;
    }
    let middle = steps.get(usize::try_from(edges.0).unwrap_or(usize::MAX))?;
    let Operation::Join {
        left: starts,
        right: Input::Relation(Relation::SyntaxChild),
        left_on: from_on,
        right_on: by,
        output: edges_output,
        filter: no_filter,
        view: no_view,

        reach: None,
    } = &middle.operation
    else {
        return None;
    };
    if middle.scope != steps.get(place.0)?.scope || !no_filter.is_empty() || !no_view.is_empty() {
        return None;
    }
    let ([start], [by]) = (from_on.as_slice(), by.as_slice()) else {
        return None;
    };
    // The node is the parent of the edges its children are found by, or the child of the edge its parent is found by.
    let (direction, found) = match *by {
        ColumnIndex(0) => (AxisDirection::Child, ColumnIndex(1)),
        ColumnIndex(1) => (AxisDirection::Parent, ColumnIndex(0)),
        _ => return None,
    };
    let was = edges_output.get(usize::try_from(found_at.0).unwrap_or(usize::MAX))?;
    if (was.side, was.column) != (Side::Right, found) {
        return None;
    }
    let width = width_of(steps, *starts)?;
    let columns = neighbour_columns(
        output,
        view,
        edges_output,
        StartingRow {
            width,
            start: *start,
            found,
        },
    )?;
    Some(Operation::Axis {
        from: *starts,
        edges: Input::Relation(Relation::SyntaxChild),
        domain: Input::Relation(Relation::SyntaxTree),
        direction,
        id: *start,
        parent: ColumnIndex(0),
        child: ColumnIndex(1),
        domain_id: ColumnIndex(0),
        domain_filter: filter.clone(),
        domain_view: Vec::new(),
        output: columns,
    })
}

/// The steps with each lookup of the elements a filter admits among the children or the parent of a node made one axis.
#[must_use]
pub(crate) fn fuse_neighbours(mut steps: Vec<PlanStep>) -> Vec<PlanStep> {
    for place in 0..steps.len() {
        if let Some(fused) = fuse_neighbour(&steps, StepIndex(place))
            && let Some(step) = steps.get_mut(place)
        {
            step.operation = fused;
        }
    }
    steps
}
