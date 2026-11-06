//! The fake of the `row_sink` port: what the core's tests stand in for it with.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, PoisonError};

use super::super::errors::RowSinkError;
use super::super::interface::RowSinkPort;
use super::super::types::{LentCell, Summary};
use ryo_values::domain::plan_parts::types::Heading;
use ryo_values::domain::records::types::Unread;
use ryo_values::domain::row_texts::types::Cell;

/// How the answer ends.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Ends {
    /// Well.
    #[default]
    Well,
    /// Whoever reads the rows has stopped.
    Closed,
    /// The rows cannot be written.
    Failed,
}

/// What the core's tests stand in for `rows` with: it counts the rows it is given and keeps none.
#[derive(Debug, Default)]
pub struct RowSinkFake {
    /// How many rows it was given.
    pub rows: AtomicUsize,
    /// How the answer ends.
    pub ends: Ends,
}

impl RowSinkPort for RowSinkFake {
    fn begin(&self, _columns: &[Heading]) {}

    fn row(&self, _cells: &mut dyn Iterator<Item = LentCell<'_>>) {
        let _rows_before = self.rows.fetch_add(1, Ordering::Relaxed);
    }

    fn end(&self, _summary: Summary, _unread: &[Unread]) -> Result<(), RowSinkError> {
        match self.ends {
            Ends::Well => Ok(()),
            Ends::Closed => Err(RowSinkError::Closed),
            Ends::Failed => Err(RowSinkError::Failed {
                reason: "the fake was told to fail".to_owned(),
            }),
        }
    }
}

/// What a test stands in for `rows` with to count the rows each rule gave: it keeps nothing else, and once a rule has been seen counting
/// allocates nothing.
#[derive(Debug, Default)]
pub struct TallyRowSinkFake {
    /// How many rows each rule gave, by the rule's id, the first cell of a row, a text cell.
    pub counts: Mutex<BTreeMap<String, usize>>,
}

impl RowSinkPort for TallyRowSinkFake {
    fn begin(&self, _columns: &[Heading]) {}

    fn row(&self, cells: &mut dyn Iterator<Item = LentCell<'_>>) {
        let Some(LentCell::Valued(Cell::Text(rule))) = cells.next() else {
            return;
        };
        let mut counts = self.counts.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(count) = counts.get_mut(rule.0) {
            *count += 1;
        } else {
            let _: Option<usize> = counts.insert(rule.0.to_owned(), 1);
        }
    }

    fn end(&self, _summary: Summary, _unread: &[Unread]) -> Result<(), RowSinkError> {
        Ok(())
    }
}

/// What a test stands in for `rows` with to measure what is written: it adds up the bytes the cells it is given hold, a file or a
/// text by the bytes of its text and a number or a span by its own size, and keeps nothing else.
#[derive(Debug, Default)]
pub struct BytesRowSinkFake {
    /// How many bytes the cells of the rows come to.
    pub bytes: AtomicUsize,
}

impl RowSinkPort for BytesRowSinkFake {
    fn begin(&self, _columns: &[Heading]) {}

    fn row(&self, cells: &mut dyn Iterator<Item = LentCell<'_>>) {
        let held = cells
            .map(|cell| match cell {
                LentCell::Valued(Cell::File(text) | Cell::Text(text)) => text.0.len(),
                LentCell::Valued(Cell::Number(number)) => size_of_val(&number),
                LentCell::Valued(Cell::Span(span)) => size_of_val(&span),
                LentCell::Empty => 0,
            })
            .sum::<usize>();
        let _bytes_before = self.bytes.fetch_add(held, Ordering::Relaxed);
    }

    fn end(&self, _summary: Summary, _unread: &[Unread]) -> Result<(), RowSinkError> {
        Ok(())
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
