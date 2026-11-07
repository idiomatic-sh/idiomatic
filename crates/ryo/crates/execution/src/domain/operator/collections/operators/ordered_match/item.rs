//! FIFO pairing over engine-prepared indexes with owned textual equality keys.
use super::super::interface::OperatorsItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::types::{
    OperatorContract, PreparedInputs, PrimitiveLease, RowSink,
};
use crate::domain::operator::contract::failure::FailureLatch;
use crate::domain::table_model::types::RepresentationInput as BorrowedRows;
use ryo_operators::inbound::use_cases::operators::types::OperatorCall;
use ryo_operators::domain::ordered_match::types::OrderedMatchDriver;
use ryo_operators::domain::contract::types::PortableOperatorFault;
use ryo_operators::domain::contract::errors::ContractError;
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_operators::types::OperationKind;
use ryo_values::domain::query_bindings::types::ParameterBinding;

struct Pairing<'a, 'input, 'engine> {
    inputs: &'a [BorrowedRows<'input>],
    ordered: &'a ryo_derived::domain::contract::types::OrderedMatching,
    left_buffer: &'a mut Vec<Value>,
    right_buffer: &'a mut Vec<Value>,
    lease: &'a mut PrimitiveLease<'engine>,
    sink: &'a mut dyn RowSink,
    error: FailureLatch,
}
impl OrderedMatchDriver for Pairing<'_, '_, '_> {
    fn left_len(&self) -> ryo_values::domain::rows::types::RowCount {
        ryo_values::domain::rows::types::RowCount(self.ordered.left().len())
    }
    fn right_len(&self) -> ryo_values::domain::rows::types::RowCount {
        ryo_values::domain::rows::types::RowCount(self.ordered.right().len())
    }
    fn compare_keys(&self, left: ryo_values::domain::rows::types::RowIndex, right: ryo_values::domain::rows::types::RowIndex) -> std::cmp::Ordering {
        self.ordered.left()[left.0]
            .keys()
            .cmp(self.ordered.right()[right.0].keys())
    }
    fn right_order_before_or_equal(&self, left: ryo_values::domain::rows::types::RowIndex, right: ryo_values::domain::rows::types::RowIndex) -> ryo_operators::domain::ordered_match::types::KeyOrder {
        if self.ordered.right()[right.0].order() <= self.ordered.left()[left.0].order() {
            ryo_operators::domain::ordered_match::types::KeyOrder::BeforeOrEqual
        } else {
            ryo_operators::domain::ordered_match::types::KeyOrder::After
        }
    }
    fn visit_pair(
        &mut self,
        left: ryo_values::domain::rows::types::RowIndex,
        right: ryo_values::domain::rows::types::RowIndex,
        project: &mut dyn FnMut(&[Value], &[Value]) -> Result<(), PortableOperatorFault>,
    ) -> Result<(), PortableOperatorFault> {
        let pair = self
            .error
            .run(|| {
                let left = self.inputs[0]
                    .read(self.ordered.left()[left.0].row(), self.left_buffer)
                    .ok_or(OperatingError::ShapeMismatch)?;
                let right = self.inputs[1]
                    .read(self.ordered.right()[right.0].row(), self.right_buffer)
                    .ok_or(OperatingError::ShapeMismatch)?;
                Ok(project(left, right))
            })
            .map_err(|()| PortableOperatorFault::Interrupted)?;
        pair
    }
    fn emit(&mut self, row: &[Value]) -> Result<(), ContractError> {
        self.error
            .run(|| self.sink.row(self.lease, row))
            .map_err(|()| ContractError::Interrupted)
    }
}

pub(crate) struct OrderedMatchItem;
impl OperatorsItem for OrderedMatchItem {
    fn prepare(
        &self,
        operation: &Operation,
        inputs: &[crate::domain::operation_schema::types::InputShape<'_>],
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
        inputs
            .first()
            .ok_or(OperatingError::MissingInput { position: 1 })?;
        inputs
            .get(1)
            .ok_or(OperatingError::MissingInput { position: 2 })?;
        let ordered = lease
            .views
            .resources
            .ordered()?
            .ok_or(OperatingError::WrongBinding)?;
        let mut left_buffer = std::mem::take(&mut lease.workspace.row);
        let mut right_buffer = std::mem::take(&mut lease.workspace.other);
        let mut projected = std::mem::take(&mut lease.workspace.captured);
        let semantics = lease.semantics();
        let mut driver = Pairing {
            inputs,
            ordered,
            left_buffer: &mut left_buffer,
            right_buffer: &mut right_buffer,
            lease: &mut *lease,
            sink: emit,
            error: FailureLatch::default(),
        };
        let answer = semantics.operators(OperatorCall::OrderedMatch {
            output,
            driver: &mut driver,
            target: &mut projected,
        });
        let result =
            crate::domain::operator::contract::types::written_result(answer, driver.error.take());
        drop(driver);
        lease.workspace.row = left_buffer;
        lease.workspace.other = right_buffer;
        lease.workspace.captured = projected;
        result
    }
}
