//! The `lowering` service: the steps of a plan that read only the tree of a file are lowered, once for the plan, to run over the
//! positions of the elements of the file: no row of values is made between them, a value is made only where a comparison, a key
//! or the step that takes the rows asks for one. A step with an input outside the fragment is not lowered, and runs as an operator.

use ryo_language::domain::relations::types::{CellKind, Relation};

use super::constants::{EDGE, ELEMENT};
use super::types::{
    ColumnFollowing, Counting, ElementPart, LoweredColumn, LoweredShape, Lowering, RowPosition,
    RowWidth,
};
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_cells::types::TreeForm;
use ryo_values::domain::plan_parts::service::step_in;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input, JoinColumn, Side, StepId};
use ryo_values::domain::plans::types::{Operation, Plan, StepScope};

/// What the column `column` of the elements of a tree holds.
fn element_field(column: ColumnIndex) -> ElementPart {
    match ELEMENT.get(usize::try_from(column.0).unwrap_or(usize::MAX)) {
        Some(part) => *part,
        // A column past the element's own is one a step added, which is no field of the element.
        None => ElementPart::Absent,
    }
}

/// What the column `column` of the edges of a tree holds.
fn edge_field(column: ColumnIndex) -> ElementPart {
    match EDGE.get(usize::try_from(column.0).unwrap_or(usize::MAX)) {
        Some(part) => *part,
        // A column past the edge's own is one a step added, which is no field of the edge.
        None => ElementPart::Absent,
    }
}

/// The column of a tree that the column `column` of the view of it is.
pub(crate) fn viewed(view: &[ColumnIndex], column: ColumnIndex) -> ColumnIndex {
    match view.get(usize::try_from(column.0).unwrap_or(usize::MAX)) {
        Some(moved) => *moved,
        // A view names only the columns it moves: a column it does not name is the tree's own.
        None => column,
    }
}

/// The kind of value a field holds.
pub(crate) fn kind_of(field: ElementPart) -> CellKind {
    match field {
        ElementPart::Node | ElementPart::Parent => CellKind::Node,
        ElementPart::Kind | ElementPart::Text | ElementPart::Role => CellKind::Symbol,
        ElementPart::File => CellKind::File,
        ElementPart::Span => CellKind::Span,
        ElementPart::Ordinal => CellKind::Number,
        ElementPart::Absent => CellKind::Flag,
    }
}

/// The columns a step keeps, from the columns it reads: all of them when none is named.
fn picked(kept: &[ColumnIndex], columns: &[LoweredColumn]) -> Option<Vec<LoweredColumn>> {
    if kept.is_empty() {
        return Some(columns.to_vec());
    }
    kept.iter()
        .map(|each| {
            columns
                .get(usize::try_from(each.0).unwrap_or(usize::MAX))
                .copied()
        })
        .collect()
}

/// The shape of the lowered step `input` is, when it is one.
fn earlier(shapes: &[Option<LoweredShape>], input: Input) -> Option<&LoweredShape> {
    match input {
        Input::Step(id) => shapes
            .get(usize::try_from(id.0).unwrap_or(usize::MAX))?
            .as_ref(),
        Input::Relation(_) => None,
    }
}

/// Whether the column holds the id of an element, which a row can be followed from.
fn follows(shape: &LoweredShape, column: ColumnIndex) -> ColumnFollowing {
    if shape
        .columns
        .get(usize::try_from(column.0).unwrap_or(usize::MAX))
        .is_some_and(|each| matches!(each.field, ElementPart::Node | ElementPart::Parent))
    {
        ColumnFollowing::Follows
    } else {
        ColumnFollowing::DoesNotFollow
    }
}

/// The columns of a join, from the columns of its sides: those of the left are as they are, those of the right are `right`.
fn joined(
    output: &[JoinColumn],
    left: &[LoweredColumn],
    right: impl Fn(ColumnIndex) -> Option<LoweredColumn>,
) -> Option<Vec<LoweredColumn>> {
    output
        .iter()
        .map(|each| match each.side {
            Side::Left => left
                .get(usize::try_from(each.column.0).unwrap_or(usize::MAX))
                .copied(),
            Side::Right => right(each.column),
        })
        .collect()
}

/// What a join is when it is lowered: the left step with an element, an edge or the rows of another lowered step.
fn join_shape(operation: &Operation, shapes: &[Option<LoweredShape>]) -> Option<LoweredShape> {
    let Operation::Join {
        left,
        right,
        left_on,
        right_on,
        output,
        filter,
        view,
    } = operation
    else {
        return None;
    };
    let left = earlier(shapes, *left)?;
    if left_on.len() != right_on.len() || left_on.is_empty() {
        return None;
    }
    let slot = left.width.0;
    match right {
        Input::Relation(relation @ (Relation::SyntaxTree | Relation::SyntaxChild)) => {
            let ([key], [on]) = (left_on.as_slice(), right_on.as_slice()) else {
                return None;
            };
            // An element is joined on its id; an edge on its parent or its child.
            let (keyed, field): (_, fn(ColumnIndex) -> ElementPart) =
                if *relation == Relation::SyntaxTree {
                    (viewed(view, *on) == ColumnIndex(0), element_field)
                } else {
                    (matches!(viewed(view, *on), ColumnIndex(0 | 1)), edge_field)
                };
            if follows(left, *key) != ColumnFollowing::Follows || !keyed {
                return None;
            }
            Some(LoweredShape {
                width: RowWidth(slot + 1),
                columns: joined(output, &left.columns, |column| {
                    Some(LoweredColumn {
                        slot: RowPosition(slot),
                        field: field(viewed(view, column)),
                    })
                })?,
            })
        }
        Input::Step(_) => {
            let right = earlier(shapes, *right)?;
            if !filter.is_empty() || !view.is_empty() {
                return None;
            }
            Some(LoweredShape {
                width: RowWidth(slot + right.width.0),
                columns: joined(output, &left.columns, |column| {
                    let each = right
                        .columns
                        .get(usize::try_from(column.0).unwrap_or(usize::MAX))?;
                    Some(LoweredColumn {
                        slot: RowPosition(slot + each.slot.0),
                        field: each.field,
                    })
                })?,
            })
        }
        Input::Relation(_) => None,
    }
}

/// What a walk is when it is lowered: the step it walks from, with the element it reaches.
fn axis_shape(operation: &Operation, shapes: &[Option<LoweredShape>]) -> Option<LoweredShape> {
    let Operation::Axis {
        from,
        edges,
        domain,
        id,
        parent,
        child,
        domain_id,
        domain_filter,
        domain_view,
        output,
        ..
    } = operation
    else {
        return None;
    };
    let from = earlier(shapes, *from)?;
    if *edges != Input::Relation(Relation::SyntaxChild)
        || (*parent, *child) != (ColumnIndex(0), ColumnIndex(1))
        || follows(from, *id) != ColumnFollowing::Follows
    {
        return None;
    }
    let slot = from.width.0;
    let reached = match domain {
        Input::Relation(Relation::SyntaxTree) => {
            if viewed(domain_view, *domain_id) != ColumnIndex(0) {
                return None;
            }
            TreeForm::Elements
        }
        Input::Step(_) => {
            let among = earlier(shapes, *domain)?;
            if !domain_filter.is_empty()
                || !domain_view.is_empty()
                || follows(among, *domain_id) != ColumnFollowing::Follows
            {
                return None;
            }
            TreeForm::Edges
        }
        Input::Relation(_) => return None,
    };
    let mut columns = if output.is_empty() {
        let mut each = from.columns.clone();
        each.push(LoweredColumn {
            slot: RowPosition(slot),
            field: ElementPart::Node,
        });
        each
    } else {
        joined(output, &from.columns, |column| {
            Some(LoweredColumn {
                slot: RowPosition(slot),
                field: if reached == TreeForm::Elements {
                    element_field(viewed(domain_view, column))
                } else {
                    edge_field(column)
                },
            })
        })?
    };
    columns.shrink_to_fit();
    Some(LoweredShape {
        width: RowWidth(slot + 1),
        columns,
    })
}

/// What a step is when it is lowered: its row and its columns.
fn shape_of(operation: &Operation, shapes: &[Option<LoweredShape>]) -> Option<LoweredShape> {
    match operation {
        Operation::Select {
            from: Input::Relation(Relation::SyntaxTree),
            kept,
            ..
        } => {
            let all: Vec<LoweredColumn> = ELEMENT
                .iter()
                .map(|field| LoweredColumn {
                    slot: RowPosition(0),
                    field: *field,
                })
                .collect();
            Some(LoweredShape {
                width: RowWidth(1),
                columns: picked(kept, &all)?,
            })
        }
        Operation::Select { from, kept, .. } => {
            let input = earlier(shapes, *from)?;
            Some(LoweredShape {
                width: input.width,
                columns: picked(kept, &input.columns)?,
            })
        }
        Operation::Project { from, columns, .. } => {
            let input = earlier(shapes, *from)?;
            Some(LoweredShape {
                width: input.width,
                columns: picked(columns, &input.columns)?,
            })
        }
        Operation::Join { .. } => join_shape(operation, shapes),
        Operation::Axis { .. } => axis_shape(operation, shapes),
        Operation::Antijoin {
            left,
            right,
            left_on,
            right_on,
        } => {
            let left = earlier(shapes, *left)?;
            if earlier(shapes, *right).is_none()
                || left_on.len() != right_on.len()
                || left_on.is_empty()
            {
                return None;
            }
            Some(left.clone())
        }
        Operation::Union { left, right } => {
            let (left, right) = (earlier(shapes, *left)?, earlier(shapes, *right)?);
            let fields = |shape: &LoweredShape| {
                shape
                    .columns
                    .iter()
                    .map(|each| each.field)
                    .collect::<Vec<_>>()
            };
            if fields(left) != fields(right) {
                return None;
            }
            Some(LoweredShape {
                width: RowWidth(left.columns.len().max(1)),
                columns: left
                    .columns
                    .iter()
                    .enumerate()
                    .map(|(slot, each)| LoweredColumn {
                        slot: RowPosition(slot),
                        field: each.field,
                    })
                    .collect(),
            })
        }
        Operation::Extend { .. }
        | Operation::Intersection { .. }
        | Operation::Difference { .. }
        | Operation::GroupCount { .. }
        | Operation::Replace { .. }
        | Operation::OrderedMatch { .. }
        | Operation::GroupDigest { .. }
        | Operation::Capture { .. }
        | Operation::Rank { .. }
        | Operation::Words { .. }
        | Operation::Closure { .. }
        | Operation::Cycles { .. }
        | Operation::Rows { .. }
        | Operation::Hosted { .. } => None,
    }
}

/// The steps of the plan that run over the positions of a tree: each per-file step that reads only the tree and the steps lowered
/// before it, by operators this service can run.
#[must_use]
pub(crate) fn lower(plan: &Plan) -> Lowering {
    let mut shapes: Vec<Option<LoweredShape>> = Vec::with_capacity(plan.steps.len());
    let mut counting = Vec::with_capacity(plan.steps.len());
    for step in &plan.steps {
        let shape = if step.scope == StepScope::PerFile {
            shape_of(&step.operation, &shapes)
        } else {
            None
        };
        let counts = step.scope == StepScope::PerFile
            && matches!(
                &step.operation,
                Operation::GroupCount { from, keys, distinct }
                    if earlier(&shapes, *from).is_some_and(|input| {
                        let width = input.columns.len();
                        keys.iter().chain(distinct.iter()).all(|each| usize::try_from(each.0).is_ok_and(|at| at < width))
                    })
            );
        counting.push(if counts {
            Counting::Counts
        } else {
            Counting::DoesNotCount
        });
        shapes.push(shape);
    }
    Lowering { shapes, counting }
}

/// The shape of the lowered step `id`.
///
/// # Errors
///
/// `NotRun` when the step is not lowered.
pub(crate) fn shape_at(lowering: &Lowering, id: StepId) -> Result<&LoweredShape, OperatingError> {
    step_in(&lowering.shapes, id)
        .and_then(Option::as_ref)
        .ok_or(OperatingError::NotRun)
}

/// The step an input is.
///
/// # Errors
///
/// `NotAStep` when the input is a relation.
pub(crate) fn step_of(input: Input) -> Result<StepId, OperatingError> {
    match input {
        Input::Step(id) => Ok(id),
        Input::Relation(_) => Err(OperatingError::NotAStep),
    }
}
