//! Match physical source bytes inside a checked span and append exact occurrence spans.
use super::super::interface::OperatorsItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::failure::FailureLatch;
use crate::domain::operator::contract::types::{
    OperatorContract, PreparedInputs, PrimitiveLease, RowSink,
};
use ryo_language::domain::symbols::service::query_intern_derived;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_operators::inbound::use_cases::operators::interface::{
    OperatorCall, PortableOperatorFault, SemanticInterrupted, SourceMatchesInput, TextRowDriver,
};
use ryo_relations::domain::relations::types::Value;
use ryo_relations::domain::terms::types::NameText;
use ryo_values::domain::plans::types::{Operation, OperationKind};
use ryo_values::domain::query_bindings::types::ParameterBinding;

struct Delivery<'a, 'engine> {
    lease: &'a mut PrimitiveLease<'engine>,
    sink: &'a mut dyn RowSink,
    error: FailureLatch,
}
impl TextRowDriver for Delivery<'_, '_> {
    fn intern(&mut self, text: &str) -> Value {
        Value::Symbol(query_intern_derived(
            &mut self.lease.derived,
            ByteSlice(self.lease.views.source.0),
            NameText(text),
        ))
    }
    fn emit(&mut self, row: &[Value]) -> Result<(), SemanticInterrupted> {
        self.error
            .run(|| self.sink.row(self.lease, row))
            .map_err(|()| SemanticInterrupted)
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
                source,
                base: checked.base,
                text: checked.text,
                line: checked.line,
                regex,
                capture: capture.as_ref().map(|name| name.0.as_ref()),
                capture_text: *capture_text == ryo_values::domain::plans::types::TextOutput::Appended,
                capture_line: *capture_line == ryo_values::domain::plans::types::LineOutput::Appended,
            },
            driver: &mut driver,
            target: &mut joined,
        });
        let result = match answer {
            Err(PortableOperatorFault::MissingCapture) if !driver.error.failed() => {
                Err(OperatingError::InvalidRegex {
                    reason: format!(
                        "missing named capture `{}`",
                        capture.as_ref().map_or("", |name| name.0.as_ref())
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
