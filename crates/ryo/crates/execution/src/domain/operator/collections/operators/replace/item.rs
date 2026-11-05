//! Append a literal replacement using the engine-acquired regular expression.
use super::super::interface::OperatorsItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::service::minted;
use crate::domain::operator::contract::types::{
    OperatorContract, PreparedInputs, PrimitiveLease, RowSink,
};
use crate::domain::table_texts::service::datum;
use crate::domain::table_texts::types::{Datum, Texts};
use ryo_language::domain::symbols::service::query_intern_derived;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_relations::domain::terms::types::NameText;
use ryo_values::domain::plans::types::{Operation, OperationKind};
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::rows::service::cell;
pub(crate) struct ReplaceItem;
impl OperatorsItem for ReplaceItem {
    fn prepare(
        &self,
        operation: &Operation,
        inputs: &[crate::domain::operation_schema::types::InputShape<'_>],
        bindings: &[ParameterBinding],
    ) -> Result<OperatorContract, OperatingError> {
        crate::domain::operator::contract::service::prepare_for(
            OperationKind::Replace,
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
        let Operation::Replace {
            text, replacement, ..
        } = operation
        else {
            return Err(OperatingError::WrongOperation);
        };
        let texts = Texts {
            symbols: lease.derived.as_ref(),
            source: lease.views.source,
            paths: lease.views.paths,
            arenas: lease.views.arenas,
        };
        let Datum::Text(value) = datum(cell(row, *text)?, &texts) else {
            return Err(OperatingError::ShapeMismatch);
        };
        let regex = lease.views.regex.ok_or(OperatingError::WrongBinding)?;
        let mut changed = std::mem::take(&mut lease.workspace.text);
        let answer = lease.semantics().operators(
            ryo_operators::inbound::use_cases::operators::interface::OperatorCall::Replace {
                regex,
                text: value.0,
                replacement: &replacement.0,
                target: &mut changed,
            },
        );
        let replacement_result =
            crate::domain::operator::contract::types::written_result(answer, None);
        if let Err(error) = replacement_result {
            lease.workspace.text = changed;
            return Err(error);
        }
        let made = query_intern_derived(
            &mut lease.derived,
            ByteSlice(lease.views.source.0),
            NameText(&changed),
        );
        lease.workspace.text = changed;
        let after = lease.views.phase;
        let appended = minted(lease.derived.as_ref(), made, after);
        let mut joined = std::mem::take(&mut lease.workspace.row);
        let result = lease
            .append(row, std::slice::from_ref(&appended), &mut joined)
            .and_then(|()| emit.row(lease, &joined));
        lease.workspace.row = joined;
        result
    }
}
