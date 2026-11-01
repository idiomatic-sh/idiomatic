//! The `operation_schema` service: output column algebra with no row values, source bytes or execution storage.

use super::types::InputSchema;
use super::types::bindings::value::BindingSchema;
use super::types::traversal::value::TraversalSchema;
use crate::domain::operating::{errors::OperatingError, types::InputPlace};
use crate::domain::table_model::errors::TableModelError;
use crate::domain::table_model::service::{check_columns, column_in};
use crate::domain::table_model::types::ParameterBinding;
use ryo_language::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::service::parameter_in;
use ryo_values::domain::plan_parts::types::{ColumnIndex, JoinColumn, Side};
use ryo_values::domain::plans::types::Operation;

/// Read one input's metadata in the same input order as execution.
pub(crate) fn schema_input<'a>(
    inputs: &[InputSchema<'a>],
    place: InputPlace,
) -> Result<InputSchema<'a>, OperatingError> {
    inputs
        .get(place.0)
        .copied()
        .ok_or(OperatingError::MissingInput {
            position: place.0 + 1,
        })
}

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
                .ok_or_else(|| TableModelError::NoSuchColumn { index: column.0 }.into())
        })
        .collect()
}

/// A fused join's logical RHS remains the original closure projection, without executing edges.
pub(crate) fn reach_kinds(
    kinds: &[CellKind],
    reach: &ryo_values::domain::plan_parts::types::ReachLookup,
) -> Result<Vec<CellKind>, OperatingError> {
    let mut columns = reach.keys.clone();
    columns.extend([reach.source, reach.destination]);
    projected(&projected(kinds, &columns)?, &reach.columns)
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
                Side::Right => (
                    right,
                    view.map_or(column.column, |view| {
                        crate::domain::tree_join::service::source_column(view, column.column)
                    }),
                ),
            };
            column_in(kinds, source).copied().ok_or_else(|| {
                TableModelError::NoSuchColumn {
                    index: column.column.0,
                }
                .into()
            })
        })
        .collect()
}

/// Derive an operation's ordered output schema from metadata alone.
///
/// This function cannot read rows, source bytes, symbols or execution scratch.
/// Binding/table-key and structural errors remain explicit; data errors such as
/// invalid spans and duplicate order values remain execution's responsibility.
pub(crate) fn schema_of(
    operation: &Operation,
    inputs: &[InputSchema<'_>],
    bindings: &[ParameterBinding],
) -> Result<Vec<CellKind>, OperatingError> {
    let first = || schema_input(inputs, InputPlace(0));
    let second = || schema_input(inputs, InputPlace(1));
    match operation {
        Operation::Select {
            comparisons, kept, ..
        } => {
            let input = first()?;
            let answer = if kept.is_empty() {
                input.kinds.to_vec()
            } else {
                projected(input.kinds, kept)?
            };
            BindingSchema::new(bindings).predicates(comparisons, input.kinds)?;
            Ok(answer)
        }
        Operation::Project { columns, .. } => projected(first()?.kinds, columns),
        Operation::Extend { values, .. } => {
            BindingSchema::new(bindings).extended(values, first()?.kinds.to_vec())
        }
        Operation::Join { .. } => join_schema(operation, inputs, bindings),
        Operation::Union { .. } | Operation::Intersection { .. } | Operation::Difference { .. } => {
            let (left, right) = (first()?, second()?);
            if left.kinds != right.kinds {
                return Err(OperatingError::ShapeMismatch);
            }
            Ok(left.kinds.to_vec())
        }
        Operation::Antijoin {
            left_on, right_on, ..
        } => {
            let (left, right) = (first()?, second()?);
            check_columns(left.kinds, left_on)?;
            check_columns(right.kinds, right_on)?;
            Ok(left.kinds.to_vec())
        }
        Operation::GroupCount { keys, distinct, .. } => {
            grouped(first()?.kinds, keys, *distinct, &[CellKind::Number])
        }
        Operation::GroupDigest { keys, over, .. } => {
            grouped(first()?.kinds, keys, Some(*over), &[CellKind::Number; 2])
        }
        Operation::Capture { path, pattern, .. } => {
            let input = first()?;
            check_columns(input.kinds, &[*path])?;
            let mut answer = input.kinds.to_vec();
            answer.extend(std::iter::repeat_n(
                CellKind::Symbol,
                memchr::memchr_iter(b'{', pattern.0.as_bytes()).count(),
            ));
            Ok(answer)
        }
        Operation::Rank { column, sets, .. } => {
            let input = first()?;
            let ParameterBinding::TextSets(_) =
                parameter_in(bindings, *sets).ok_or(OperatingError::Unbound)?
            else {
                return Err(OperatingError::WrongBinding);
            };
            appended(input.kinds, &[*column], CellKind::Number)
        }
        Operation::Replace { text, .. } | Operation::Words { text, .. } => {
            appended(first()?.kinds, &[*text], CellKind::Symbol)
        }
        Operation::Axis { .. } | Operation::OrderedMatch { .. } | Operation::OrderedDfs { .. } => {
            TraversalSchema::new(inputs).of(operation, bindings)
        }
        Operation::Closure {
            keys,
            source,
            destination,
            ..
        } => {
            let input = first()?;
            check_columns(input.kinds, &[*source, *destination])?;
            let mut answer = projected(input.kinds, keys)?;
            answer.extend(projected(input.kinds, &[*source, *destination])?);
            Ok(answer)
        }
        Operation::Cycles {
            source,
            destination,
            ..
        } => projected(first()?.kinds, &[*source, *destination]),
        Operation::Rows { parameter, keys } => BindingSchema::new(bindings).rows(*parameter, keys),
        Operation::Hosted { .. } => Err(OperatingError::WrongOperation),
        Operation::SpanBetween { start, end, .. } => {
            let input = first()?;
            if projected(input.kinds, &[*start, *end])? != [CellKind::Span; 2] {
                return Err(OperatingError::ShapeMismatch);
            }
            appended(input.kinds, &[], CellKind::Span)
        }
    }
}

/// Check join keys and native RHS view before deriving its ordered output projection.
fn join_schema(
    operation: &Operation,
    inputs: &[InputSchema<'_>],
    bindings: &[ParameterBinding],
) -> Result<Vec<CellKind>, OperatingError> {
    let Operation::Join {
        left_on,
        right_on,
        output,
        filter,
        view,
        reach,
        ..
    } = operation
    else {
        return Err(OperatingError::WrongOperation);
    };
    let (left, right) = (
        schema_input(inputs, InputPlace(0))?,
        schema_input(inputs, InputPlace(1))?,
    );
    let virtual_right = reach
        .as_ref()
        .map(|reach| reach_kinds(right.kinds, reach))
        .transpose()?;
    let right_kinds = virtual_right.as_deref().unwrap_or(right.kinds);
    let view = if reach.is_some() {
        None
    } else {
        right.tree.map(|_| view.as_slice())
    };
    let answer = joined(left.kinds, right_kinds, view, output)?;
    for column in right_on {
        check_columns(
            right_kinds,
            &[view.map_or(*column, |view| {
                crate::domain::tree_join::service::source_column(view, *column)
            })],
        )?;
    }
    check_columns(left.kinds, left_on)?;
    BindingSchema::new(bindings).predicates(filter, right_kinds)?;
    Ok(answer)
}

/// Group key projection, with the over-column validated before the keys.
fn grouped(
    kinds: &[CellKind],
    keys: &[ColumnIndex],
    over: Option<ColumnIndex>,
    tail: &[CellKind],
) -> Result<Vec<CellKind>, OperatingError> {
    if let Some(column) = over {
        check_columns(kinds, &[column])?;
    }
    let mut output = projected(kinds, keys)?;
    output.extend_from_slice(tail);
    Ok(output)
}

/// A scalar/text/span operator appends one kind while retaining every existing column.
fn appended(
    kinds: &[CellKind],
    needed: &[ColumnIndex],
    kind: CellKind,
) -> Result<Vec<CellKind>, OperatingError> {
    check_columns(kinds, needed)?;
    let mut output = kinds.to_vec();
    output.push(kind);
    Ok(output)
}
