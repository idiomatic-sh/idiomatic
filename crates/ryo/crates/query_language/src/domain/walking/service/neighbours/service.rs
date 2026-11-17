//! The `walking` service's neighbour part: a join of an element to its parent or child and of that to the elements, made one
//! lookup of the syntax tree.

use ryo_relations::domain::relations::collections::relations::syntax_child::item::SyntaxChildColumn;
use ryo_relations::domain::relations::collections::relations::syntax_tree::item::SyntaxTreeColumn;
use ryo_relations::domain::relations::types::{Column, Relation};

use super::super::super::types::{EdgeSource, StartingRow};
use super::super::{edge_column, edge_source, width_of};
use crate::domain::rewriting::types::StepIndex;
use ryo_values::domain::plan_parts::service::column_place;
use ryo_values::domain::plan_parts::service::view_source;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Direction as AxisDirection;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plan_parts::types::JoinColumn;
use ryo_values::domain::plan_parts::types::Side;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::plans::types::PlanStep;

/// The columns the fused lookup gives, as the axis has them: those the second join kept, found in the elements the axis reaches, or
/// in the row it starts from, `width` columns wide, whose node is at `start`. The edge the first join made is not made: its parent
/// or child is the node the row holds or the element reached, and its file is the file of the element; its ordinal and field are not.
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
    let element = |column| JoinColumn {
        side: Side::Right,
        column: column_place(Column::SyntaxTree(column)),
    };
    let mut columns = Vec::new();
    for kept in output {
        columns.push(match kept.side {
            Side::Right => JoinColumn {
                side: Side::Right,
                column: view_source(view, kept.column)?,
            },
            Side::Left => {
                let was = edges_output.get(usize::try_from(kept.column.0).unwrap_or(usize::MAX))?;
                let source = edge_column(was.column).map(|edge| edge_source(edge, found));
                match (was.side, source) {
                    // The edge's end that is the element reached.
                    (Side::Right, Some(EdgeSource::Reached)) => element(SyntaxTreeColumn::Id),
                    // The edge's other end, which is the node the row holds.
                    (Side::Right, Some(EdgeSource::Start)) => JoinColumn {
                        side: Side::Left,
                        column: start,
                    },
                    (Side::Right, Some(EdgeSource::Element(column))) => element(column),
                    (Side::Left, _) if was.column.0 < width.0 => *was,
                    (Side::Left, _) | (Side::Right, None | Some(EdgeSource::Unmade)) => {
                        return None;
                    }
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
        fan: _,
        reach: None,
    } = &steps.get(place.0)?.operation
    else {
        return None;
    };
    let ([found_at], [element_key]) = (left_on.as_slice(), right_on.as_slice()) else {
        return None;
    };
    let element_id = column_place(Column::SyntaxTree(SyntaxTreeColumn::Id));
    if view_source(view, *element_key)? != element_id {
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
        fan: _,
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
    let (direction, found) = match edge_column(*by)? {
        SyntaxChildColumn::Parent => Some((AxisDirection::Child, SyntaxChildColumn::Child)),
        SyntaxChildColumn::Child => Some((AxisDirection::Parent, SyntaxChildColumn::Parent)),
        SyntaxChildColumn::Ordinal | SyntaxChildColumn::Field | SyntaxChildColumn::File => None,
    }?;
    let was = edges_output.get(usize::try_from(found_at.0).unwrap_or(usize::MAX))?;
    if (was.side, was.column) != (Side::Right, column_place(Column::SyntaxChild(found))) {
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
        parent: column_place(Column::SyntaxChild(SyntaxChildColumn::Parent)),
        child: column_place(Column::SyntaxChild(SyntaxChildColumn::Child)),
        domain_id: element_id,
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
