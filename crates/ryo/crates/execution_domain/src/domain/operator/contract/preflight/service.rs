//! The `preflight` service: free functions named by verb over the domain's types, with no IO and no ports.

use super::super::types::*;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operation_schema::types::InputShape;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::types::InputNumber;
use ryo_values::domain::plans::types::Operation;

pub(crate) fn empty_results(operation: &Operation, inputs: &[InputShape<'_>]) -> Vec<EmptyResult> {
    match operation {
        Operation::Antijoin { .. } => vec![EmptyResult {
            input: InputNumber(0),
            representation: EmptyRepresentation::Any,
            kinds: inputs
                .first()
                .map(|input| input.kinds.to_vec())
                .ok_or(OperatingError::MissingInput { position: 1 }),
        }],
        Operation::Axis { id, output, .. } => {
            let from = inputs
                .first()
                .ok_or(OperatingError::MissingInput { position: 1 });
            let root_kinds = (|| {
                let from = from?;
                crate::domain::table_model::service::check_columns(from.kinds, &[*id])?;
                let domain = inputs
                    .get(2)
                    .ok_or(OperatingError::MissingInput { position: 3 })?;
                let mut out = ryo_values::domain::rows::types::Table::default();
                crate::domain::tree_walk::service::walk_kinds(
                    from.kinds,
                    domain.kinds,
                    output,
                    &mut out,
                )?;
                Ok(out.kinds)
            })();
            let mut gates = vec![EmptyResult {
                input: InputNumber(0),
                representation: EmptyRepresentation::Any,
                kinds: root_kinds,
            }];
            if output.is_empty() {
                let kinds = (|| {
                    let from = inputs
                        .first()
                        .ok_or(OperatingError::MissingInput { position: 1 })?;
                    crate::domain::table_model::service::check_columns(from.kinds, &[*id])?;
                    let mut kinds = from.kinds.to_vec();
                    kinds.push(CellKind::Node);
                    Ok(kinds)
                })();
                gates.push(EmptyResult {
                    input: InputNumber(2),
                    representation: EmptyRepresentation::Computed,
                    kinds,
                });
            }
            gates
        }
        Operation::Select { .. }
        | Operation::Project { .. }
        | Operation::Extend { .. }
        | Operation::Join { .. }
        | Operation::Union { .. }
        | Operation::Intersection { .. }
        | Operation::Difference { .. }
        | Operation::GroupCount { .. }
        | Operation::GroupDigest { .. }
        | Operation::Capture { .. }
        | Operation::Rank { .. }
        | Operation::Replace { .. }
        | Operation::OrderedMatch { .. }
        | Operation::Words { .. }
        | Operation::Closure { .. }
        | Operation::OrderedDfs { .. }
        | Operation::Cycles { .. }
        | Operation::Rows { .. }
        | Operation::Hosted { .. }
        | Operation::SpanBetween { .. }
        | Operation::SourceMatches { .. }
        | Operation::RenderSyntax { .. }
        | Operation::FitsRendered { .. }
        | Operation::SpanBounds { .. } => Vec::new(),
    }
}

/// A row-lazy operand refusal does not erase the schema of an empty collected result.
pub(crate) fn output_layout(
    operation: &Operation,
    inputs: &[InputShape<'_>],
    checked: &Result<Vec<CellKind>, OperatingError>,
) -> Result<Vec<CellKind>, OperatingError> {
    let mut kinds = inputs.first().map_or(&[][..], |input| input.kinds).to_vec();
    match operation {
        Operation::Select { kept, .. } => {
            let input = inputs
                .first()
                .ok_or(OperatingError::MissingInput { position: 1 })?;
            return if kept.is_empty() {
                Ok(input.kinds.to_vec())
            } else {
                crate::domain::operation_schema::projection::service::projected(input.kinds, kept)
            };
        }
        Operation::Capture { pattern, .. } => kinds.extend(std::iter::repeat_n(
            CellKind::Symbol,
            memchr::memchr_iter(b'{', pattern.0.as_bytes()).count(),
        )),
        Operation::Rank { .. } => kinds.push(CellKind::Number),
        Operation::Words { .. } | Operation::Replace { .. } => kinds.push(CellKind::Symbol),
        Operation::SpanBetween { .. } => kinds.push(CellKind::Span),
        Operation::SpanBounds { .. } => kinds.extend([CellKind::Number; 2]),
        Operation::SourceMatches {
            base_line,
            capture,
            capture_text,
            capture_line,
            ..
        } => {
            kinds.push(CellKind::Span);
            if base_line.is_some() {
                kinds.push(CellKind::Number);
            }
            if capture.is_some() {
                kinds.push(CellKind::Span);
                if *capture_text == ryo_values::domain::query_operators::types::TextOutput::Appended
                {
                    kinds.push(CellKind::Symbol);
                }
                if *capture_line == ryo_values::domain::query_operators::types::LineOutput::Appended
                {
                    kinds.push(CellKind::Number);
                }
            }
        }
        Operation::RenderSyntax { .. } => kinds.push(CellKind::Symbol),
        Operation::FitsRendered { .. } => {}
        Operation::Rows { keys, .. } => return Ok(vec![CellKind::Symbol; keys.len()]),
        Operation::Antijoin { .. } => {
            return inputs
                .first()
                .map(|input| input.kinds.to_vec())
                .ok_or(OperatingError::MissingInput { position: 1 });
        }
        Operation::Project { .. }
        | Operation::Extend { .. }
        | Operation::Join { .. }
        | Operation::Union { .. }
        | Operation::Intersection { .. }
        | Operation::Difference { .. }
        | Operation::GroupCount { .. }
        | Operation::GroupDigest { .. }
        | Operation::OrderedMatch { .. }
        | Operation::Axis { .. }
        | Operation::Closure { .. }
        | Operation::OrderedDfs { .. }
        | Operation::Cycles { .. }
        | Operation::Hosted { .. } => return checked.clone(),
    }
    Ok(kinds)
}
