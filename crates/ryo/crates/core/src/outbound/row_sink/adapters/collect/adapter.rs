//! The rows port, implemented by keeping every row.

use std::sync::PoisonError;

use super::connection::CollectorConnection;
use crate::outbound::row_sink::errors::RowSinkError;
use crate::outbound::row_sink::interface::RowSinkPort;
use crate::outbound::row_sink::types::{AnswerCounts, KeptRow, LentCell};
use ryo_values::domain::plan_parts::types::Heading;
use ryo_values::domain::records::service::owned;
use ryo_values::domain::records::types::Unread;

/// The collect rows adapter.
#[derive(Debug)]
pub(crate) struct RowSinkCollectAdapter {
    /// Where the rows are kept so far, with the headings and the summary.
    into: CollectorConnection,
}

impl RowSinkCollectAdapter {
    /// Keeps the rows it is given in `into`.
    pub(crate) const fn new(into: CollectorConnection) -> Self {
        Self { into }
    }
}

impl RowSinkPort for RowSinkCollectAdapter {
    fn begin(&self, columns: &[Heading]) {
        self.into
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .columns = columns.to_vec();
    }

    fn row(&self, cells: &mut dyn Iterator<Item = LentCell<'_>>) {
        let row = KeptRow(cells.map(|cell| Option::from(cell).map(owned)).collect());
        self.into
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .rows
            .push(row);
    }

    fn end(&self, summary: AnswerCounts, unread: &[Unread]) -> Result<(), RowSinkError> {
        let mut held = self.into.0.lock().unwrap_or_else(PoisonError::into_inner);
        held.unread = unread.to_vec();
        held.summary = summary;
        Ok(())
    }
}
