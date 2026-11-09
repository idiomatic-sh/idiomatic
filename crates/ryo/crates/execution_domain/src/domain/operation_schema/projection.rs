//! Column projection metadata shared by preparation and empty continuation publication.
use crate::domain::operating::{errors::OperatingError, types::InputPlace};
use crate::domain::operation_schema::types::{InputShape, OriginSide};
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, JoinColumn, Side},
    plans::types::Operation,
    rows::{errors::RowsError, service::column_in},
};

/// Project columns without consulting a row; a repeated column remains repeated.
pub(crate) fn projected(
    kinds: &[CellKind],
    columns: &[ColumnIndex],
) -> Result<Vec<CellKind>, OperatingError> {
    columns
        .iter()
        .map(|column| {
            column_in(kinds, *column)
                .copied()
                .ok_or_else(|| RowsError::NoSuchColumn { index: column.0 }.into())
        })
        .collect()
}

/// Carry the schema of an empty first instruction through the compiler's closed
/// row-continuation family without evaluating predicates or acquiring bindings.
pub fn continuation_kinds(
    operation: &Operation,
    input: &[CellKind],
) -> Result<Vec<CellKind>, OperatingError> {
    match operation {
        Operation::Select { kept, .. } if kept.is_empty() => Ok(input.to_vec()),
        Operation::Project {
            columns,
            distinct: ryo_values::domain::plan_parts::types::RepeatedRows::Kept,
            ..
        } => projected(input, columns),
        _ => Err(OperatingError::WrongOperation),
    }
}

/// Read one input's metadata in the same input order as execution.
pub(crate) fn schema_input<'a>(
    inputs: &[InputShape<'a>],
    place: InputPlace,
) -> Result<InputShape<'a>, OperatingError> {
    inputs
        .get(place.0)
        .copied()
        .ok_or(OperatingError::MissingInput {
            position: place.0 + 1,
        })
}

/// Output kinds of a two-input projection, optionally through a native tree view.
pub(crate) fn joined(
    left: &[CellKind],
    right: &[CellKind],
    view: Option<&[ColumnIndex]>,
    output: &[JoinColumn],
) -> Result<Vec<CellKind>, OperatingError> {
    output
        .iter()
        .map(|column| {
            let (kinds, source) = match column.side {
                Side::Left => (left, column.column),
                Side::Right => (right, right_column(view, column.column)),
            };
            column_in(kinds, source).copied().ok_or_else(|| {
                RowsError::NoSuchColumn {
                    index: column.column.0,
                }
                .into()
            })
        })
        .collect()
}

/// The view a join's right columns are read through: its declared view over a native tree input, none over a computed
/// input or a reach join, whose right columns are the closure's own.
pub(crate) fn right_view<'a>(
    view: &'a [ColumnIndex],
    tree: Option<crate::domain::table_cells::types::TreeForm>,
    reach: Option<&ryo_values::domain::plan_parts::types::ReachLookup>,
) -> Option<&'a [ColumnIndex]> {
    if reach.is_some() {
        None
    } else {
        tree.map(|_| view)
    }
}

/// The right input column a join's right column names, through its view when it has one.
pub(crate) fn right_column(view: Option<&[ColumnIndex]>, column: ColumnIndex) -> ColumnIndex {
    view.map_or(column, |view| {
        crate::domain::table_cells::service::source_column(view, column)
    })
}

/// The native tree form a step's output keeps: a select that filters and drops nothing, and a projection that keeps
/// every column in place, read their input's tree; any other step computes its rows.
pub fn tree_output(
    operation: &Operation,
    input: Option<InputShape<'_>>,
) -> Option<crate::domain::table_cells::types::TreeForm> {
    match operation {
        Operation::Select {
            comparisons, kept, ..
        } if comparisons.is_empty() && kept.is_empty() => input.and_then(|input| input.tree),
        Operation::Project {
            columns, distinct, ..
        } if *distinct == ryo_values::domain::plan_parts::types::RepeatedRows::Kept
            && input.is_some_and(|input| {
                columns.len() == input.kinds.len()
                    && columns
                        .iter()
                        .enumerate()
                        .all(|(at, column)| column.0 as usize == at)
            }) =>
        {
            input.and_then(|input| input.tree)
        }
        _ => None,
    }
}

/// Where each column of an operation's answer comes from, from where each column of its inputs comes from: the one
/// propagation of column origins. A column keeps its origin when the operation passes it on as it was in one input row
/// (select and project keep or reorder columns; the appended-column operators add columns with none; an antijoin or a
/// difference keeps its left rows). A join, an intersection and an ordered match hand each side's origins to `side`,
/// which answers what the origin is on that side of this operation, or none. Every other operation makes new rows, and
/// its columns have no origin.
pub fn output_origins<T: Copy>(
    operation: &Operation,
    inputs: &[InputShape<'_>],
    origins: &[&[Option<T>]],
    side: &mut dyn FnMut(OriginSide, Option<T>) -> Option<T>,
) -> Vec<Option<T>> {
    // ignore: idiomatic.arch/no-unwrap-or-on-lookup - an input a step lacks (a deferred schema fault) has no origins: none is the answer.
    let input = |place: InputPlace| origins.get(place.0).copied().unwrap_or(&[]);
    let pick =
        |from: &[Option<T>], column: ColumnIndex| from.get(column.0 as usize).copied().flatten();
    let mut answer = input(InputPlace(0)).to_vec();
    match operation {
        Operation::Select { kept, .. } => {
            if !kept.is_empty() {
                answer = kept
                    .iter()
                    .map(|column| pick(input(InputPlace(0)), *column))
                    .collect();
            }
        }
        Operation::Project { columns, .. } => {
            answer = columns
                .iter()
                .map(|column| pick(input(InputPlace(0)), *column))
                .collect();
        }
        Operation::Extend { values, .. } => answer.extend(values.iter().map(|_| None)),
        Operation::SpanBetween { .. } | Operation::RenderSyntax { .. } => answer.push(None),
        Operation::FitsRendered { .. } => {}
        Operation::SourceMatches {
            base_line,
            capture,
            capture_text,
            capture_line,
            ..
        } => {
            answer.push(None);
            if base_line.is_some() {
                answer.push(None);
            }
            if capture.is_some() {
                answer.push(None);
                if *capture_text == ryo_values::domain::query_operators::types::TextOutput::Appended
                {
                    answer.push(None);
                }
                if *capture_line == ryo_values::domain::query_operators::types::LineOutput::Appended
                {
                    answer.push(None);
                }
            }
        }
        Operation::SpanBounds { .. } => answer.extend([None, None]),
        Operation::Antijoin { .. } | Operation::Difference { .. } => {}
        Operation::Join {
            output,
            view,
            reach,
            ..
        } => {
            let view = right_view(
                view,
                inputs.get(1).and_then(|right| right.tree),
                reach.as_ref(),
            );
            answer = output
                .iter()
                .map(|column| match column.side {
                    Side::Left => side(OriginSide::Left, pick(input(InputPlace(0)), column.column)),
                    Side::Right if reach.is_some() => None,
                    Side::Right => side(
                        OriginSide::Right,
                        pick(input(InputPlace(1)), right_column(view, column.column)),
                    ),
                })
                .collect();
        }
        Operation::Intersection { .. } => {
            answer = answer
                .into_iter()
                .map(|origin| side(OriginSide::Left, origin))
                .collect();
        }
        Operation::Union { .. }
        | Operation::GroupCount { .. }
        | Operation::Replace { .. }
        | Operation::OrderedMatch { .. }
        | Operation::OrderedDfs { .. }
        | Operation::GroupDigest { .. }
        | Operation::Capture { .. }
        | Operation::Rank { .. }
        | Operation::Words { .. }
        | Operation::Axis { .. }
        | Operation::Closure { .. }
        | Operation::Cycles { .. }
        | Operation::Rows { .. }
        | Operation::Hosted { .. } => answer = Vec::new(),
    }
    answer
}
