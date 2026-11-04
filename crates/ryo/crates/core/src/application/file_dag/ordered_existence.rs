//! One invocation uses the existing owner and injected meaning; no retained strategy state.
use super::relations::{PublishedInput, RelationOwner};
use crate::domain::operator::contract::{
    failure::FailureLatch, types::portable_fault, views::BorrowedRows,
};
use crate::domain::{
    operating::errors::OperatingError,
    ordered_existence::types::{Extremum, ResolvedShape},
};
use ryo_operators::domain::contract::types::{
    JoinInterrupted, OperatorCall, OrderedExistenceDriver, OrderedExistenceScratch,
    PartitionExtremum,
};
use ryo_relations::domain::relations::types::{Integer, Value};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Side},
    rows::types::{RowIndex, Table},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AdmissionDecline {
    UnprovenPopulation,
}
pub(crate) enum StrategyOutcome {
    Executed(PublishedInput),
    Declined(AdmissionDecline),
}

struct Driver<'a, 'c> {
    rows: BorrowedRows<'a>,
    output: &'a [ryo_values::domain::plan_parts::types::JoinColumn],
    row: &'a mut Vec<Value>,
    context: &'a mut crate::domain::operating::types::OperatorContext<'c>,
    emit: &'a mut dyn crate::domain::operating::types::Emit,
    failure: FailureLatch,
}
impl Driver<'_, '_> {
    fn cell(&self, row: usize, column: ColumnIndex) -> Result<Value, OperatingError> {
        self.rows
            .cell(RowIndex(row), column)
            .ok_or(OperatingError::ShapeMismatch)
    }
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

/// Eligibility decline happens BEFORE semantic output. Faults never fall back.
pub(super) fn evaluate(
    owner: &RelationOwner,
    port: &dyn crate::outbound::operators::interface::OperatorsPort,
    shape: &ResolvedShape,
    source: &PublishedInput,
    rows: BorrowedRows<'_>,
    output_kinds: &[ryo_relations::domain::relations::types::CellKind],
    context: &mut crate::domain::operating::types::OperatorContext<'_>,
    room: &mut crate::domain::operating::types::Room,
) -> Result<StrategyOutcome, OperatingError> {
    let Some(admitted) = owner.admit_number(source, shape.source, &shape.shape.number, rows)?
    else {
        return Ok(StrategyOutcome::Declined(
            AdmissionDecline::UnprovenPopulation,
        ));
    };
    let _pin = admitted.publication();
    let mut table = Table::default();
    table.kinds.extend_from_slice(output_kinds);
    let mut output_state = super::execution::output::State::take(context.scratch, context.memo);
    let result = output_state.run(&mut super::execution::output::collect(&mut table), |emit| {
        let mut driver = Driver {
            rows,
            output: &shape.output,
            row: &mut room.scratch.row,
            context,
            emit,
            failure: FailureLatch::default(),
        };
        let result = port.operators(OperatorCall::OrderedExistence {
            partition: &shape.shape.partition,
            order: admitted.column,
            extremum: match shape.shape.extremum {
                Extremum::Maximum => PartitionExtremum::Maximum,
                Extremum::Minimum => PartitionExtremum::Minimum,
            },
            driver: &mut driver,
            scratch: &mut OrderedExistenceScratch::default(),
        });
        if let Some(error) = driver.failure.take() {
            return Err(error);
        }
        result.map_err(portable_fault).map(|_| ())
    });
    output_state.restore(context.scratch, context.memo);
    result?;
    owner.computed(table).map(StrategyOutcome::Executed)
}
