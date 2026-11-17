//! The `walking` service: free functions named by verb over the descents and neighbour lookups of the syntax tree made one walk.

pub(crate) mod neighbours;

use ryo_relations::domain::relations::collections::relations::syntax_child::item::SyntaxChildColumn;
use ryo_relations::domain::relations::collections::relations::syntax_tree::item::SyntaxTreeColumn;
use ryo_relations::domain::relations::types::{Column, Relation};

use super::types::{EdgeSource, ElementDemand};
use crate::domain::rewriting::types::StepIndex;
use crate::domain::widths::service::width_of;
use crate::domain::widths::types::InputWidth;
use ryo_values::domain::plan_parts::service::view_source;
use ryo_values::domain::plan_parts::service::{column_place, relation_column};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Direction as AxisDirection;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plan_parts::types::JoinColumn;
use ryo_values::domain::plan_parts::types::Operand;
use ryo_values::domain::plan_parts::types::PlanComparison;
use ryo_values::domain::plan_parts::types::QueryLiteral;
use ryo_values::domain::plan_parts::types::Side;
use ryo_values::domain::plan_parts::types::Test;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::plans::types::PlanStep;

/// The column of the syntax tree's edges at `index`, when they have that many columns.
fn edge_column(index: ColumnIndex) -> Option<SyntaxChildColumn> {
    let Some(Column::SyntaxChild(column)) = relation_column(Relation::SyntaxChild, index) else {
        return None;
    };
    Some(column)
}

/// Whether the comparisons ask only that an element of the syntax tree be a node: a node has no text.
fn only_nodes(comparisons: &[PlanComparison]) -> ElementDemand {
    if matches!(
        comparisons,
        [PlanComparison {
            test: Test::Equal,
            left: Operand::Column(column),
            right: Operand::Literal(QueryLiteral::Text(text)),
            ..
        }] if *column == column_place(Column::SyntaxTree(SyntaxTreeColumn::Text))
            && text.0.is_empty()
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
                column: view_source(view, kept.column)?,
            },
            Side::Left => {
                let was = edges_output.get(usize::try_from(kept.column.0).unwrap_or(usize::MAX))?;
                let source =
                    edge_column(was.column).map(|edge| edge_source(edge, SyntaxChildColumn::Child));
                match (was.side, source) {
                    // Only the child the edge reached is kept: the walk makes no other column of the edge.
                    (Side::Right, Some(EdgeSource::Reached)) => JoinColumn {
                        side: Side::Right,
                        column: column_place(Column::SyntaxTree(SyntaxTreeColumn::Id)),
                    },
                    (Side::Left, _) if was.column.0 < width.0 => *was,
                    (Side::Left | Side::Right, _) => return None,
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
        fan: _,
        reach: None,
    } = &steps.get(place.0)?.operation
    else {
        return None;
    };
    let ([left_key], [right_key]) = (left_on.as_slice(), right_on.as_slice()) else {
        return None;
    };
    let element_id = column_place(Column::SyntaxTree(SyntaxTreeColumn::Id));
    let edge_parent = column_place(Column::SyntaxChild(SyntaxChildColumn::Parent));
    let edge_child = column_place(Column::SyntaxChild(SyntaxChildColumn::Child));
    if view_source(view, *right_key)? != element_id {
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
        fan: _,
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
        parent,
        child,
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
        || (*parent, *child) != (edge_parent, edge_child)
        || !no_filter.is_empty()
        || !no_view.is_empty()
        || !no_output.is_empty()
        || by_parent.as_slice() != [edge_parent]
        || only_nodes(domain_filter) == ElementDemand::More
        || view_source(domain_view, *domain_id)? != element_id
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
    if (found.side, found.column) != (Side::Right, edge_child) {
        return None;
    }
    let columns = descent_columns(output, view, edges_output, width)?;
    Some(Operation::Axis {
        from: *from,
        edges: Input::Relation(Relation::SyntaxChild),
        domain: Input::Relation(Relation::SyntaxTree),
        direction: AxisDirection::Deep,
        id: *id,
        parent: edge_parent,
        child: edge_child,
        domain_id: element_id,
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

/// Where the fused lookup finds the column `edge` of the edge it does not make, when the element it reaches is the edge's end `found`.
fn edge_source(edge: SyntaxChildColumn, found: SyntaxChildColumn) -> EdgeSource {
    match edge {
        SyntaxChildColumn::Parent | SyntaxChildColumn::Child if edge == found => {
            EdgeSource::Reached
        }
        SyntaxChildColumn::Parent | SyntaxChildColumn::Child => EdgeSource::Start,
        // The edge is in the file of both its ends.
        SyntaxChildColumn::File => EdgeSource::Element(SyntaxTreeColumn::File),
        // The element reached holds neither the place nor the role of the edge that reached it.
        SyntaxChildColumn::Ordinal | SyntaxChildColumn::Field => EdgeSource::Unmade,
    }
}
