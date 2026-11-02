//! FIFO pairing over engine-prepared indexes with owned textual equality keys.
use super::super::interface::OperatorsItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::types::{
    OperatorContract, PreparedInputs, PrimitiveLease, RowSink,
};
use crate::domain::table_model::service::cell;
use crate::domain::table_model::types::ParameterBinding;
use ryo_values::domain::plan_parts::types::Side;
use ryo_values::domain::plans::types::{Operation, OperationKind};
pub(crate) struct OrderedMatchItem;
impl OperatorsItem for OrderedMatchItem {
    fn prepare(
        &self,
        operation: &Operation,
        inputs: &[crate::domain::operation_schema::types::InputSchema<'_>],
        bindings: &[ParameterBinding],
    ) -> Result<OperatorContract, OperatingError> {
        crate::domain::operator::contract::service::prepare_for(
            OperationKind::OrderedMatch,
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
        let Operation::OrderedMatch { output, .. } = operation else {
            return Err(OperatingError::WrongOperation);
        };
        let PreparedInputs::Views(inputs) = inputs else {
            return Err(OperatingError::WrongOperation);
        };
        let (left, right) = (
            inputs
                .first()
                .ok_or(OperatingError::MissingInput { position: 1 })?,
            inputs
                .get(1)
                .ok_or(OperatingError::MissingInput { position: 2 })?,
        );
        let ordered = lease.views.ordered.ok_or(OperatingError::WrongBinding)?;
        let mut left_buffer = std::mem::take(lease.workspace.row);
        let mut right_buffer = std::mem::take(lease.workspace.other);
        let mut projected = std::mem::take(lease.workspace.captured);
        let result = (|| {
            let mut next_right = 0;
            for opened in &ordered.left {
                while let Some(closed) = ordered.right.get(next_right) {
                    if closed.keys < opened.keys
                        || closed.keys == opened.keys && closed.order <= opened.order
                    {
                        next_right += 1;
                    } else {
                        break;
                    }
                }
                let Some(closed) = ordered.right.get(next_right) else {
                    break;
                };
                if closed.keys != opened.keys {
                    continue;
                }
                let left_row = left
                    .read(opened.row, &mut left_buffer)
                    .ok_or(OperatingError::ShapeMismatch)?;
                let right_row = right
                    .read(closed.row, &mut right_buffer)
                    .ok_or(OperatingError::ShapeMismatch)?;
                projected.clear();
                for column in output {
                    projected.push(cell(
                        match column.side {
                            Side::Left => left_row,
                            Side::Right => right_row,
                        },
                        column.column,
                    )?);
                }
                emit.row(lease, &projected)?;
                next_right += 1;
            }
            Ok(())
        })();
        *lease.workspace.row = left_buffer;
        *lease.workspace.other = right_buffer;
        *lease.workspace.captured = projected;
        result
    }
}
