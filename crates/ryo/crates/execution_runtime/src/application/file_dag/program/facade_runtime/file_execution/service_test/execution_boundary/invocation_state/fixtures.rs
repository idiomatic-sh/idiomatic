//! The support of the `invocation_state` cases.
use super::super::super::super::service::*;
use super::super::super::super::*;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operating::types::Memo;
use ryo_execution_domain::domain::operator::types::Emit as RowSink;
#[cfg(test)]
use ryo_execution_domain::domain::operator::types::OperatorContext;
use ryo_relations::domain::relations::types::Value as Cell;

pub(super) struct MemoSink {
    pub(super) starts: Vec<usize>,
    pub(super) rows: Vec<Cell>,
}

impl RowSink for MemoSink {
    fn row(
        &mut self,
        context: &mut OperatorContext<'_>,
        row: &[Cell],
    ) -> Result<(), OperatingError> {
        self.starts.push(context.memo.len());
        context.memo.push(Memo::Holds);
        self.rows.extend_from_slice(row);
        Ok(())
    }
}
