//! Match physical source bytes inside a checked span and append exact occurrence spans.
use super::super::interface::OperatorsItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::failure::FailureLatch;
use crate::domain::operator::contract::types::{
    OperatorContract, PreparedInputs, PrimitiveLease, RowSink,
};
use crate::domain::operator::contract::views::ValidatedSource;
use ryo_language::domain::symbols::service::query_intern_derived;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_operators::inbound::use_cases::operators::types::OperatorCall;
use ryo_operators::domain::contract::types::PortableOperatorFault;
use ryo_operators::domain::contract::errors::ContractError;
use ryo_operators::domain::source_matches::types::SourceMatchesInput;
use ryo_operators::domain::source_matches::types::TextRowDriver;
use ryo_relations::domain::relations::types::Value;
use ryo_relations::domain::terms::types::NameText;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_operators::types::OperationKind;
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_operators::domain::source_matches::types::{BaseLine, SourceBase};
use ryo_relations::domain::relations::constants::NO_SPAN;
use ryo_relations::domain::relations::types::Integer;
use ryo_values::domain::files::types::SourceView;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::row_texts::types::CellText;
use ryo_values::domain::rows::service::cell;

struct Delivery<'a, 'engine> {
    lease: &'a mut PrimitiveLease<'engine>,
    sink: &'a mut dyn RowSink,
    error: FailureLatch,
}
impl TextRowDriver for Delivery<'_, '_> {
    fn intern(&mut self, text: ryo_values::domain::row_texts::types::CellText<'_>) -> Value {
        Value::Symbol(query_intern_derived(
            &mut self.lease.derived,
            ByteSlice(self.lease.views.source.0),
            NameText(text.0),
        ))
    }
    fn emit(&mut self, row: &[Value]) -> Result<(), ContractError> {
        self.error
            .run(|| self.sink.row(self.lease, row))
            .map_err(|()| ContractError::Interrupted)
    }
}

pub(crate) struct SourceMatchesItem;
impl OperatorsItem for SourceMatchesItem {
    fn prepare(
        &self,
        operation: &Operation,
        inputs: &[crate::domain::operation_schema::types::InputShape<'_>],
        bindings: &[ParameterBinding],
    ) -> Result<OperatorContract, OperatingError> {
        crate::domain::operator::contract::service::prepare_for(
            OperationKind::SourceMatches,
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
        let Operation::SourceMatches {
            capture,
            capture_text,
            capture_line,
            ..
        } = operation
        else {
            return Err(OperatingError::WrongOperation);
        };
        let source = lease.views.source.0;
        let checked = lease
            .views
            .validated_source
            .ok_or(OperatingError::WrongBinding)?;
        let regex = lease.views.regex.ok_or(OperatingError::WrongBinding)?;
        let semantics = lease.semantics();
        let mut joined = std::mem::take(&mut lease.workspace.row);
        let mut driver = Delivery {
            lease,
            sink: emit,
            error: FailureLatch::default(),
        };
        let answer = semantics.operators(OperatorCall::SourceMatches {
            input: SourceMatchesInput {
                row,
                source: ryo_operators::domain::source_matches::types::LineBytes(source),
                base: checked.base,
                text: checked.text,
                line: checked.line,
                regex,
                capture: capture.as_ref().map_or(
                    ryo_operators::domain::source_matches::types::CaptureRequest::Whole,
                    |name| {
                        ryo_operators::domain::source_matches::types::CaptureRequest::Group(
                            ryo_values::domain::row_texts::types::CellText(name.0.as_ref()),
                        )
                    },
                ),
                capture_text: *capture_text,
                capture_line: *capture_line,
            },
            driver: &mut driver,
            target: &mut joined,
        });
        let result = match answer {
            Err(PortableOperatorFault::MissingCapture) if driver.error.failed() == crate::domain::operator::contract::failure::LatchState::Open => {
                Err(OperatingError::InvalidRegex {
                    reason: format!(
                        "missing named capture `{}`",
                        capture.as_ref().map(|name| &*name.0).unwrap_or_default()
                    ),
                })
            }
            other => {
                crate::domain::operator::contract::types::written_result(other, driver.error.take())
            }
        };
        driver.lease.workspace.row = joined;
        result
    }
}

/// The checked physical text of a span and where it starts in the source.
struct SourceSlice<'a> {
    base: SourceBase,
    text: CellText<'a>,
}

/// Return the physical text only after validating the whole input span. An absent
/// span carries no occurrences; malformed boundaries fail before the row emits.
fn source_slice<'a>(
    row: &[Value],
    span: ColumnIndex,
    source: SourceView<'a>,
) -> Result<Option<SourceSlice<'a>>, OperatingError> {
    let Value::Span(span) = cell(row, span)? else {
        return Err(OperatingError::ShapeMismatch);
    };
    if span == NO_SPAN {
        return Ok(None);
    }
    let start =
        usize::try_from(span.start.0).map_err(|_| OperatingError::InvalidSourceMatchSpan)?;
    let end = usize::try_from(span.end.0).map_err(|_| OperatingError::InvalidSourceMatchSpan)?;
    if start > end {
        return Err(OperatingError::InvalidSourceMatchSpan);
    }
    let bytes = source
        .0
        .get(start..end)
        .ok_or(OperatingError::InvalidSourceMatchSpan)?;
    let text =
        std::str::from_utf8(bytes).map_err(|_error| OperatingError::InvalidSourceMatchSpan)?;
    Ok(Some(SourceSlice {
        base: SourceBase(start),
        text: CellText(text),
    }))
}

/// Validate before any regex is acquired. An absent span suppresses subsequent row checks.
pub(crate) fn validate<'a>(
    row: &[Value],
    span: ColumnIndex,
    base_line: Option<ColumnIndex>,
    source: SourceView<'a>,
) -> Result<Option<ValidatedSource<'a>>, OperatingError> {
    let Some(SourceSlice { base, text }) = source_slice(row, span, source)? else {
        return Ok(None);
    };
    let line = if let Some(column) = base_line {
        let Value::Number(Integer(number)) = cell(row, column)? else {
            return Err(OperatingError::ShapeMismatch);
        };
        if number <= 0 {
            return Err(OperatingError::InvalidSourceMatchSpan);
        }
        BaseLine::Starts(Integer(number))
    } else {
        BaseLine::Unnumbered
    };
    Ok(Some(ValidatedSource { base, text, line }))
}
