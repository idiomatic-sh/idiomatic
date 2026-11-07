//! Borrowed engine resources for portable set and antijoin row meaning.
use super::failure::FailureLatch;
use super::types::{PrimitiveLease, RowSink, written_result};
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_dedup::service::first_time;
use crate::domain::table_model::types::Addition;
use ryo_operators::inbound::use_cases::operators::types::OperatorCall;
use ryo_operators::domain::set_rows::types::SetInput;
use ryo_operators::domain::set_rows::types::SetInterrupted;
use ryo_operators::domain::set_rows::types::SetKind;
use ryo_operators::domain::set_rows::types::SetRowDriver;
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::ColumnIndex;

struct EngineRows<'a, 'b> {
    lease: &'a mut PrimitiveLease<'b>,
    emit: &'a mut dyn RowSink,
    failure: FailureLatch,
}

impl SetRowDriver for EngineRows<'_, '_> {
    fn right_empty(&mut self) -> Result<bool, SetInterrupted> {
        self.failure
            .run(|| {
                let index = self
                    .lease
                    .views
                    .resources
                    .index()
                    .ok_or(OperatingError::WrongBinding)?;
                Ok(index.rows.count() == 0)
            })
            .map_err(|()| SetInterrupted)
    }

    fn contains_right(&mut self, key: &[Value]) -> Result<bool, SetInterrupted> {
        self.failure
            .run(|| {
                let index = self
                    .lease
                    .views
                    .resources
                    .index()
                    .ok_or(OperatingError::WrongBinding)?;
                Ok(index.contains(key))
            })
            .map_err(|()| SetInterrupted)
    }

    fn first_time(&mut self, row: &[Value]) -> Result<bool, SetInterrupted> {
        self.failure
            .run(|| {
                let state = self.lease.distinct()?;
                Ok(first_time(&mut state.rows, &mut state.seen, row) == Addition::Added)
            })
            .map_err(|()| SetInterrupted)
    }

    fn emit(&mut self, row: &[Value]) -> Result<(), SetInterrupted> {
        self.failure
            .run(|| self.emit.row(self.lease, row))
            .map_err(|()| SetInterrupted)
    }
}

/// The engine selects an acquired index and invocation workspace before one semantic call.
pub(crate) fn run(
    kind: SetKind,
    row: &[Value],
    left_on: &[ColumnIndex],
    lease: &mut PrimitiveLease<'_>,
    emit: &mut dyn RowSink,
) -> Result<(), OperatingError> {
    let semantics = lease.semantics();
    let mut key = std::mem::take(&mut lease.workspace.row);
    let (answer, error) = {
        let mut driver = EngineRows {
            lease,
            emit,
            failure: FailureLatch::default(),
        };
        let answer = semantics.operators(OperatorCall::SetRow {
            input: SetInput::Values {
                kind,
                row,
                left_on,
                driver: &mut driver,
                key: &mut key,
            },
        });
        (answer, driver.failure.take())
    };
    lease.workspace.row = key;
    written_result(answer, error)
}
