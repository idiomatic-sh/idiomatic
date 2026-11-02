//! Request parameter rows, interpreted without scheduling or cache capabilities.
use super::super::interface::OperatorsItem;
use crate::domain::operating::{errors::OperatingError, service::minted, types::AfterTheFiles};
use crate::domain::operation_schema::types::InputSchema;
use crate::domain::operator::contract::types::{
    OperatorContract, PreparedInputs, PrimitiveLease, RowSink,
};
use crate::domain::table_model::types::ParameterBinding;
use ryo_language::domain::relations::types::Value;
use ryo_language::domain::symbols::{service::query_intern_derived, types::ByteSlice};
use ryo_language::domain::terms::types::NameText;
use ryo_values::domain::plan_parts::service::parameter_in;
use ryo_values::domain::plans::types::{Operation, OperationKind};
pub(crate) struct RowsItem;
fn tables_rows(
    operation: &Operation,
    lease: &mut PrimitiveLease<'_>,
    row: &mut Vec<Value>,
    emit: &mut dyn RowSink,
) -> Result<(), OperatingError> {
    let Operation::Rows { parameter, keys } = operation else {
        return Err(OperatingError::WrongOperation);
    };
    let bindings = lease.views.bindings;
    let tables = match parameter_in(bindings, *parameter).ok_or(OperatingError::Unbound)? {
        ParameterBinding::Tables(tables) => tables.as_slice(),
        // An empty list says no kind of its items: it is no tables.
        ParameterBinding::Texts(items) if items.is_empty() => &[],
        ParameterBinding::Numbers(items) if items.is_empty() => &[],
        ParameterBinding::TextSets(items) if items.is_empty() => &[],
        ParameterBinding::Number(_)
        | ParameterBinding::Flag(_)
        | ParameterBinding::Text(_)
        | ParameterBinding::Numbers(_)
        | ParameterBinding::Texts(_)
        | ParameterBinding::TextSets(_) => return Err(OperatingError::WrongBinding),
    };
    let after = if lease.views.arenas.is_empty() {
        AfterTheFiles::Before
    } else {
        AfterTheFiles::After
    };
    for table in tables {
        row.clear();
        for key in keys {
            let cell = table
                .0
                .iter()
                .find(|cell| *cell.key.0 == *key.0)
                .ok_or(OperatingError::WrongBinding)?;
            let made = query_intern_derived(
                &mut lease.derived,
                ByteSlice(lease.views.source.0),
                NameText(&cell.value.0),
            );
            row.push(minted(lease.derived.as_ref(), made, after));
        }
        emit.row(lease, row)?;
    }
    Ok(())
}

impl OperatorsItem for RowsItem {
    fn prepare(
        &self,
        operation: &Operation,
        inputs: &[InputSchema<'_>],
        bindings: &[ParameterBinding],
    ) -> Result<OperatorContract, OperatingError> {
        crate::domain::operator::contract::service::prepare_for(
            OperationKind::Rows,
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
        if !matches!(inputs, PreparedInputs::RequestSource(_)) {
            return Err(OperatingError::WrongOperation);
        }
        let mut row = std::mem::take(lease.workspace.row);
        let outcome = tables_rows(operation, lease, &mut row, emit);
        *lease.workspace.row = row;
        outcome
    }
}
