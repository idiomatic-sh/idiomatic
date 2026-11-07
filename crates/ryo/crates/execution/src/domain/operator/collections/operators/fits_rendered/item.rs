//! The engine side of the width check: the semantics are the operators' `FitsRendered`.

use super::super::interface::OperatorsItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::types::{
    OperatorContract, PreparedInputs, PrimitiveLease, RowSink,
};
use crate::domain::table_texts::service::datum;
use crate::domain::table_texts::types::{Datum, Texts};
use ryo_relations::domain::relations::constants::NO_SPAN;
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_operators::types::OperationKind;
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::rows::service::cell;

pub(crate) struct FitsRenderedItem;

impl OperatorsItem for FitsRenderedItem {
    fn prepare(
        &self,
        operation: &Operation,
        inputs: &[crate::domain::operation_schema::types::InputShape<'_>],
        bindings: &[ParameterBinding],
    ) -> Result<OperatorContract, OperatingError> {
        crate::domain::operator::contract::service::prepare_for(
            OperationKind::FitsRendered,
            operation,
            inputs,
            bindings,
        )
    }

    fn evaluate(
        &self,
        operation: &Operation,
        inputs: PreparedInputs<'_>,
        lease: &mut PrimitiveLease<'_>,
        emit: &mut dyn RowSink,
    ) -> Result<(), OperatingError> {
        let PreparedInputs::Row { row, .. } = inputs else {
            return Err(OperatingError::WrongOperation);
        };
        let Operation::FitsRendered {
            texts: columns,
            literals,
            span,
            max_line,
            tab_width,
            ..
        } = operation
        else {
            return Err(OperatingError::WrongOperation);
        };
        let Value::Span(span) = cell(row, *span)? else {
            return Err(OperatingError::ShapeMismatch);
        };
        let source =
            std::str::from_utf8(lease.views.source.0).map_err(|_| OperatingError::InvalidSpan)?;
        let start = usize::try_from(span.start.0).map_err(|_| OperatingError::InvalidSpan)?;
        let end = usize::try_from(span.end.0).map_err(|_| OperatingError::InvalidSpan)?;
        if span == NO_SPAN || start > end || source.get(start..end).is_none() {
            return Err(OperatingError::InvalidSpan);
        }
        let line_start = source[..start].rfind(['\r', '\n']).map_or(0, |at| at + 1);
        let prefix = &source[line_start..start];
        let texts = Texts {
            symbols: lease.derived.as_ref(),
            source: lease.views.source,
            paths: lease.views.paths,
            arenas: lease.views.arenas,
        };
        if literals.len() != columns.len() + 1 {
            return Err(OperatingError::ShapeMismatch);
        }
        let mut rendered = std::mem::take(&mut lease.workspace.text.0);
        rendered.clear();
        for (literal, column) in literals.iter().zip(columns) {
            rendered.push_str(&literal.0);
            let Datum::Text(value) = datum(cell(row, *column)?, &texts) else {
                lease.workspace.text.0 = rendered;
                return Err(OperatingError::ShapeMismatch);
            };
            rendered.push_str(value.0);
        }
        if let Some(last) = literals.last() {
            rendered.push_str(&last.0);
        }
        let maximum = usize::try_from(max_line.0).map_err(|_| OperatingError::WrongBinding)?;
        let tab_width = usize::try_from(tab_width.0).map_err(|_| OperatingError::WrongBinding)?;
        if tab_width == 0 {
            return Err(OperatingError::WrongBinding);
        }
        let answer = lease.semantics().operators(
            ryo_operators::inbound::use_cases::operators::types::OperatorCall::FitsRendered {
                rendered: ryo_operators::domain::fits_rendered::types::CandidateText(&rendered),
                prefix: ryo_operators::domain::fits_rendered::types::LinePrefix(prefix),
                maximum: ryo_operators::domain::fits_rendered::types::LineWidth(maximum),
                tab_width: ryo_operators::domain::fits_rendered::types::TabStop(tab_width),
            },
        );
        lease.workspace.text.0 = rendered;
        match answer {
            Ok(ryo_operators::inbound::use_cases::operators::types::OperatorAnswer::Selected(
                ryo_operators::domain::select::types::RowSelection::Kept,
            )) => emit.row(lease, row),
            Ok(ryo_operators::inbound::use_cases::operators::types::OperatorAnswer::Selected(
                ryo_operators::domain::select::types::RowSelection::Dropped,
            )) => Ok(()),
            Ok(_) => Err(OperatingError::WrongOperation),
            Err(error) => Err(crate::domain::operator::contract::types::portable_fault(error)),
        }
    }
}
