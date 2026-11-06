//! The `operation_schema` service: output column algebra with no row values, source bytes or execution storage.
use super::projection::projected;

use super::types::InputShape;
use super::types::bindings::value::BindingShape;
use super::types::traversal::value::TraversalShape;
use crate::domain::operating::{errors::OperatingError, types::InputPlace};
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::service::parameter_in;
use ryo_values::domain::plan_parts::types::{ColumnIndex, JoinColumn, Side};
use ryo_values::domain::plans::service::inputs_of;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::rows::errors::RowsError;
use {
    crate::domain::table_model::service::check_columns,
    ryo_values::domain::rows::service::column_in,
};

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
                RowsError::NoSuchColumn {
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
    inputs: &[InputShape<'_>],
    bindings: &[ParameterBinding],
) -> Result<Vec<CellKind>, OperatingError> {
    let first = || schema_input(inputs, InputPlace(0));
    let second = || schema_input(inputs, InputPlace(1));
    match operation {
        Operation::Select { .. } => select_schema(operation, inputs, bindings),
        Operation::Project { .. } => project_schema(operation, inputs, bindings),
        Operation::Extend { values, .. } => {
            BindingShape::new(bindings).extended(values, first()?.kinds.to_vec())
        }
        Operation::Join { .. } => join_schema(operation, inputs, bindings),
        Operation::Union { .. } => union_schema(operation, inputs, bindings),
        Operation::Intersection { .. } | Operation::Difference { .. } => {
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
            TraversalShape::new(inputs).of(operation, bindings)
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
        Operation::Rows { parameter, keys } => BindingShape::new(bindings).rows(*parameter, keys),
        Operation::Hosted { .. } => Ok(schema_input(inputs, InputPlace(0))?.kinds.to_vec()),
        Operation::SpanBetween { start, end, .. } => {
            let input = first()?;
            if projected(input.kinds, &[*start, *end])? != [CellKind::Span; 2] {
                return Err(OperatingError::ShapeMismatch);
            }
            appended(input.kinds, &[], CellKind::Span)
        }
        Operation::SourceMatches { .. } => source_matches_schema(operation, inputs, bindings),
        Operation::RenderSyntax { node, .. } => {
            let input = first()?;
            if projected(input.kinds, &[*node])? != [CellKind::Node] {
                return Err(OperatingError::ShapeMismatch);
            }
            appended(input.kinds, &[], CellKind::Symbol)
        }
        Operation::FitsRendered {
            texts,
            literals,
            span,
            ..
        } => {
            let input = first()?;
            if texts.is_empty()
                || literals.len() != texts.len() + 1
                || texts.iter().any(|text| {
                    projected(input.kinds, &[*text]).ok() != Some(vec![CellKind::Symbol])
                })
                || projected(input.kinds, &[*span])? != [CellKind::Span]
            {
                return Err(OperatingError::ShapeMismatch);
            }
            Ok(input.kinds.to_vec())
        }
        Operation::SpanBounds { .. } => span_bounds_schema(operation, inputs, bindings),
    }
}

/// Select's metadata contract lives beside the other operations' schema algebra.
/// No row or source payload is read here.
pub(crate) fn select_schema(
    operation: &Operation,
    inputs: &[InputShape<'_>],
    bindings: &[ParameterBinding],
) -> Result<Vec<CellKind>, OperatingError> {
    let Operation::Select {
        comparisons, kept, ..
    } = operation
    else {
        return Err(OperatingError::WrongOperation);
    };
    let input = schema_input(inputs, InputPlace(0))?;
    let answer = if kept.is_empty() {
        input.kinds.to_vec()
    } else {
        projected(input.kinds, kept)?
    };
    BindingShape::new(bindings).predicates(comparisons, input.kinds)?;
    Ok(answer)
}

pub(crate) fn project_schema(
    operation: &Operation,
    inputs: &[InputShape<'_>],
    _: &[ParameterBinding],
) -> Result<Vec<CellKind>, OperatingError> {
    let Operation::Project { columns, .. } = operation else {
        return Err(OperatingError::WrongOperation);
    };
    projected(schema_input(inputs, InputPlace(0))?.kinds, columns)
}

pub(crate) fn union_schema(
    operation: &Operation,
    inputs: &[InputShape<'_>],
    _: &[ParameterBinding],
) -> Result<Vec<CellKind>, OperatingError> {
    if !matches!(operation, Operation::Union { .. }) {
        return Err(OperatingError::WrongOperation);
    }
    let (left, right) = (
        schema_input(inputs, InputPlace(0))?,
        schema_input(inputs, InputPlace(1))?,
    );
    if left.kinds != right.kinds {
        return Err(OperatingError::ShapeMismatch);
    }
    Ok(left.kinds.to_vec())
}

pub(crate) fn source_matches_schema(
    operation: &Operation,
    inputs: &[InputShape<'_>],
    _: &[ParameterBinding],
) -> Result<Vec<CellKind>, OperatingError> {
    let Operation::SourceMatches {
        span,
        base_line,
        capture,
        capture_text,
        capture_line,
        ..
    } = operation
    else {
        return Err(OperatingError::WrongOperation);
    };
    let input = schema_input(inputs, InputPlace(0))?;
    if projected(input.kinds, &[*span])? != [CellKind::Span] {
        return Err(OperatingError::ShapeMismatch);
    }
    if let Some(base_line) = base_line
        && projected(input.kinds, &[*base_line])? != [CellKind::Number]
    {
        return Err(OperatingError::ShapeMismatch);
    }
    let mut output = appended(input.kinds, &[], CellKind::Span)?;
    if base_line.is_some() {
        output.push(CellKind::Number);
    }
    if capture.is_some() {
        output.push(CellKind::Span);
        if *capture_text == ryo_values::domain::plans::types::TextOutput::Appended {
            output.push(CellKind::Symbol);
        }
        if *capture_line == ryo_values::domain::plans::types::LineOutput::Appended {
            output.push(CellKind::Number);
        }
    }
    Ok(output)
}

pub(crate) fn span_bounds_schema(
    operation: &Operation,
    inputs: &[InputShape<'_>],
    _: &[ParameterBinding],
) -> Result<Vec<CellKind>, OperatingError> {
    let Operation::SpanBounds { span, .. } = operation else {
        return Err(OperatingError::WrongOperation);
    };
    let input = schema_input(inputs, InputPlace(0))?;
    if projected(input.kinds, &[*span])? != [CellKind::Span] {
        return Err(OperatingError::ShapeMismatch);
    }
    let mut output = input.kinds.to_vec();
    output.extend([CellKind::Number, CellKind::Number]);
    Ok(output)
}

/// Check join keys and native RHS view before deriving its ordered output projection.
pub(crate) fn join_schema(
    operation: &Operation,
    inputs: &[InputShape<'_>],
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
    BindingShape::new(bindings).predicates(filter, right_kinds)?;
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
