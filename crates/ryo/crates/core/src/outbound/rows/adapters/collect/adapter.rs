//! The rows port, implemented by keeping every row.

use std::sync::PoisonError;

use crate::outbound::rows::errors::RowsError;
use crate::outbound::rows::interface::RowsPort;
use crate::outbound::rows::types::{Collector, LentCell, Row, Summary};
use ryo_values::domain::plan_parts::types::Heading;
use ryo_values::domain::records::service::owned;
use ryo_values::domain::records::types::Unread;

/// The collect rows adapter.
#[derive(Debug)]
pub(crate) struct RowsCollectAdapter {
    /// Where the rows are kept so far, with the headings and the summary.
    into: Collector,
}

impl RowsCollectAdapter {
    /// Keeps the rows it is given in `into`.
    pub(crate) const fn new(into: Collector) -> Self {
        Self { into }
    }
}

impl RowsPort for RowsCollectAdapter {
    fn begin(&self, columns: &[Heading]) {
        self.into
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .columns = columns.to_vec();
    }

    fn row(&self, cells: &mut dyn Iterator<Item = LentCell<'_>>) {
        let row = Row(cells.map(|cell| Option::from(cell).map(owned)).collect());
        self.into
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .rows
            .push(row);
    }

    fn end(&self, summary: Summary, unread: &[Unread]) -> Result<(), RowsError> {
        let mut held = self.into.0.lock().unwrap_or_else(PoisonError::into_inner);
        held.unread = unread.to_vec();
        held.summary = summary;
        Ok(())
    }
}
