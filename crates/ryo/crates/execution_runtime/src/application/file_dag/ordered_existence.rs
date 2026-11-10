//! One invocation uses the existing owner and injected meaning; no retained strategy state.
use ryo_execution_domain::domain::operating::service::portable_fault;
use ryo_execution_domain::domain::operator::contract::failure::FailureLatch;
use ryo_execution_domain::domain::table_model::types::RepresentationInput;
use ryo_execution_domain::domain::{
    operating::errors::OperatingError, self_join_recogniser::types::Extremum,
};
use ryo_operators::domain::equality_join::errors::EqualityJoinError;
use ryo_operators::domain::ordered_existence::types::OrderedExistenceDriver;
use ryo_operators::domain::ordered_existence::types::OrderedExistenceScratch;
use ryo_operators::domain::ordered_existence::types::PartitionExtremum;
use ryo_operators::inbound::use_cases::operators::types::OperatorCall;
use ryo_relations::domain::relations::types::{Integer, Value};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Side},
    rows::types::{RowCount, RowIndex},
};

struct Driver<'a, 'c> {
    rows: RepresentationInput<'a>,
    output: &'a [ryo_values::domain::plan_parts::types::JoinColumn],
    row: &'a mut Vec<Value>,
    context: &'a mut ryo_execution_domain::domain::operator::types::OperatorContext<'c>,
    emit: &'a mut dyn ryo_execution_domain::domain::operator::types::Emit,
    failure: FailureLatch,
    work: &'a ryo_execution_domain::domain::operator::contract::types::WorkCounters,
}
impl OrderedExistenceDriver for Driver<'_, '_> {
    fn len(&self) -> RowCount {
        self.rows.count()
    }
    fn key_cell(&mut self, row: RowIndex, column: ColumnIndex) -> Result<Value, EqualityJoinError> {
        let rows = self.rows;
        self.failure
            .run(|| rows.cell(row, column).ok_or(OperatingError::ShapeMismatch))
            .map_err(|_| EqualityJoinError::Interrupted)
    }
    fn number(&mut self, row: RowIndex, column: ColumnIndex) -> Result<Integer, EqualityJoinError> {
        let rows = self.rows;
        self.failure
            .run(|| match rows.cell(row, column) {
                Some(Value::Number(value)) => Ok(value),
                _ => Err(OperatingError::ShapeMismatch),
            })
            .map_err(|_| EqualityJoinError::Interrupted)
    }
    fn emit_dominated(
        &mut self,
        row: RowIndex,
        witness: RowIndex,
    ) -> Result<(), EqualityJoinError> {
        self.work.found(
            ryo_values::domain::plan_parts::types::InputNumber(0),
            ryo_execution_domain::domain::operator::contract::types::FoundRows(1),
        );
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
                        rows.cell(at, column.column)
                            .ok_or(OperatingError::ShapeMismatch)?,
                    );
                }
                emit.row(context, logical)
            })
            .map_err(|_| EqualityJoinError::Interrupted)
    }
}

/// The owner has already bound the exact numeric source. This function only
/// drives the borrowed semantic algorithm and never publishes or schedules.
pub(super) fn evaluate(
    semantics: &dyn ryo_execution_domain::domain::operating::types::PrimitiveSemantics,
    shape: &ryo_execution_domain::domain::executable_program::types::OrderedExistence,
    admitted: &ryo_execution_store::application::file_dag::relations::number::service::AdmittedNumber<'_>,
    rows: RepresentationInput<'_>,
    context: &mut ryo_execution_domain::domain::operator::types::OperatorContext<'_>,
    row: &mut Vec<Value>,
    scratch: &mut OrderedExistenceScratch,
    work: &ryo_execution_domain::domain::operator::contract::types::WorkCounters,
    emit: &mut dyn ryo_execution_domain::domain::operator::types::Emit,
) -> Result<(), OperatingError> {
    let _pin = admitted.publication();
    if admitted.column() != shape.order {
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
        order: admitted.column(),
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
        Ok(ryo_operators::inbound::use_cases::operators::types::OperatorAnswer::Written) => Ok(()),
        Ok(_) => Err(OperatingError::WrongOperation),
        Err(error) => Err(portable_fault(error)),
    }
}
