//! The answer of `query run` as the sink of the edge takes it: the headings, then each row's cells, then the counts and the files nothing
//! read.

use ryo_core::outbound::row_sink::interface::RowSinkPort;
use ryo_core::outbound::row_sink::types::{AnswerCounts, LentCell};
use ryo_core::outbound::row_sink::errors::RowSinkError;
use ryo_relations::domain::relations::types::Integer;
use ryo_sdk::linter::{Cell, Slot, Unreadable};
use ryo_sdk::query::RunReport;
use ryo_values::domain::plan_parts::types::Heading;
use ryo_values::domain::records::types::{Unread, Unreadable as KeptUnreadable};
use ryo_values::domain::row_texts::types::{Cell as RowCell, CellText, RowTotal};
use ryo_values::domain::stored_facts::types::ReadFailure;

use super::schemas::RunSchemaError;

/// The cell a row of the report holds, as the sink takes it: nothing for a value that reads as nothing.
fn cell(slot: &Slot) -> LentCell<'_> {
    match slot {
        Slot::Is(Cell::File(path)) => LentCell::Valued(RowCell::File(CellText(&path.0))),
        Slot::Is(Cell::Text(text)) => LentCell::Valued(RowCell::Text(CellText(&text.0))),
        Slot::Is(Cell::Number(number)) => LentCell::Valued(RowCell::Number(Integer(number.0))),
        Slot::Is(Cell::Span(span)) => LentCell::Valued(RowCell::Span(*span)),
        _ => LentCell::Empty,
    }
}

/// Writes `report` to `sink`.
///
/// # Errors
///
/// `OutputClosed` when whoever reads the answer stopped reading, `Output` when it could not be written.
pub(super) fn write(report: &RunReport, sink: &dyn RowSinkPort) -> Result<(), RunSchemaError> {
    let columns: Vec<Heading> = report.columns.iter().map(|each| Heading(each.0.clone())).collect();
    sink.begin(&columns);
    for row in &report.rows {
        sink.row(&mut row.0.iter().map(cell));
    }
    let unread: Vec<Unread> = report
        .unread
        .iter()
        .map(|each| Unread {
            path: ryo_language::domain::files::types::FilePath(each.path.0.clone()),
            reason: match &each.reason {
                Unreadable::Failed(reason) => KeptUnreadable::Failed(ReadFailure(reason.0.clone())),
                _ => KeptUnreadable::NoLanguage,
            },
        })
        .collect();
    sink.end(
        AnswerCounts { total: RowTotal(report.total.0), shown: RowTotal(report.shown.0) },
        &unread,
    )
    .map_err(|error| match error {
        RowSinkError::Closed => RunSchemaError::OutputClosed,
        RowSinkError::Failed { reason } => RunSchemaError::Output(reason),
    })
}
