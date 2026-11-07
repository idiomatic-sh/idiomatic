//! One invocation uses the existing owner and injected meaning; no retained strategy state.
use crate::domain::operator::contract::{
    failure::FailureLatch, types::portable_fault, views::BorrowedRows,
};
use crate::domain::{operating::errors::OperatingError, self_join_recogniser::types::Extremum};
use ryo_operators::inbound::use_cases::operators::interface::{
    JoinInterrupted, OperatorCall, OrderedExistenceDriver, OrderedExistenceScratch,
    PartitionExtremum,
};
use ryo_relations::domain::relations::types::{Integer, Value};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Side},
    rows::types::RowIndex,
};

struct Driver<'a, 'c> {
    rows: BorrowedRows<'a>,
    output: &'a [ryo_values::domain::plan_parts::types::JoinColumn],
    row: &'a mut Vec<Value>,
    context: &'a mut crate::domain::operating::types::OperatorContext<'c>,
    emit: &'a mut dyn crate::domain::operating::types::Emit,
    failure: FailureLatch,
    work: &'a crate::domain::operator::contract::types::WorkCounters,
}
impl OrderedExistenceDriver for Driver<'_, '_> {
    fn len(&self) -> usize {
        self.rows.count()
    }
    fn key_cell(&mut self, row: usize, column: ColumnIndex) -> Result<Value, JoinInterrupted> {
        let rows = self.rows;
        self.failure
            .run(|| {
                rows.cell(RowIndex(row), column)
                    .ok_or(OperatingError::ShapeMismatch)
            })
            .map_err(|_| JoinInterrupted)
    }
    fn number(&mut self, row: usize, column: ColumnIndex) -> Result<Integer, JoinInterrupted> {
        let rows = self.rows;
        self.failure
            .run(|| match rows.cell(RowIndex(row), column) {
                Some(Value::Number(value)) => Ok(value),
                _ => Err(OperatingError::ShapeMismatch),
            })
            .map_err(|_| JoinInterrupted)
    }
    fn emit_dominated(&mut self, row: usize, witness: usize) -> Result<(), JoinInterrupted> {
        self.work.found(0, 1);
        let rows = self.rows;
        let logical = &mut self.row;
        let output = self.output;
        let context = &mut self.context;
        let emit = &mut self.emit;
        self.failure
            .run(|| {
                // Real witness reconstructs the original Join coordinates. Project,
                // DISTINCT and antijoin remain ordinary nodes with their own ports.
                logical.clear();
                for column in output {
                    let at = if column.side == Side::Left {
                        row
                    } else {
                        witness
                    };
                    logical.push(
                        rows.cell(RowIndex(at), column.column)
                            .ok_or(OperatingError::ShapeMismatch)?,
                    );
                }
                emit.row(context, logical)
            })
            .map_err(|_| JoinInterrupted)
    }
}

/// The owner has already bound the exact numeric source. This function only
/// drives the borrowed semantic algorithm and never publishes or schedules.
pub(super) fn evaluate(
    semantics: &dyn crate::domain::operator::contract::types::PrimitiveSemantics,
    shape: &crate::domain::executable_program::types::OrderedExistence,
    admitted: &super::relations::number::service::AdmittedNumber<'_>,
    rows: BorrowedRows<'_>,
    context: &mut crate::domain::operating::types::OperatorContext<'_>,
    row: &mut Vec<Value>,
    scratch: &mut OrderedExistenceScratch,
    work: &crate::domain::operator::contract::types::WorkCounters,
    emit: &mut dyn crate::domain::operating::types::Emit,
) -> Result<(), OperatingError> {
    let _pin = admitted.publication();
    if admitted.column != shape.order {
        return Err(OperatingError::WrongOperation);
    }
    let mut driver = Driver {
        rows,
        output: &shape.output,
        row,
        context,
        emit,
        failure: FailureLatch::default(),
        work,
    };
    let result = semantics.operators(OperatorCall::OrderedExistence {
        partition: &shape.partition,
        order: admitted.column,
        extremum: match shape.extremum {
            Extremum::Maximum => PartitionExtremum::Maximum,
            Extremum::Minimum => PartitionExtremum::Minimum,
        },
        driver: &mut driver,
        scratch,
    });
    if let Some(error) = driver.failure.take() {
        return Err(error);
    }
    match result {
        Ok(ryo_operators::inbound::use_cases::operators::interface::OperatorAnswer::Written) => {
            Ok(())
        }
        Ok(_) => Err(OperatingError::WrongOperation),
        Err(error) => Err(portable_fault(error)),
    }
}
