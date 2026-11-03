//! The `records` service: free functions named by verb over what a run found of each file, with no IO and no ports.

use crate::domain::files::types::CorpusEntry;
use crate::domain::files::types::Nanoseconds;
use crate::domain::records::types::Freshness;
use crate::domain::records::types::Record;
use crate::domain::records::types::{
    Cell, CellText, RenderedText, RowsTextAt, RowsTextView, TextRange,
};

/// Whether the file the inventory lists is the one the record is of, by what the system says of it.
#[must_use]
pub fn is_as_it_was(record: &Record, entry: &CorpusEntry) -> Freshness {
    if record.executable == entry.executable
        && record.size == entry.size
        && record.modified == entry.modified
        && entry.modified != Nanoseconds(0)
    {
        Freshness::AsItWas
    } else {
        Freshness::Changed
    }
}

/// The cell `kept` is, its text lent from `text`, the text of its rows: a range `text` does not hold lends nothing.
#[must_use]
pub fn lent(text: RowsTextView<'_>, kept: Cell<TextRange>) -> Cell<CellText<'_>> {
    let piece = |range: TextRange| {
        CellText(
            text.0
                .get(
                    usize::try_from(range.start.0).unwrap_or(usize::MAX)
                        ..usize::try_from(range.end.0).unwrap_or(usize::MAX),
                )
                .unwrap_or_default(),
        )
    };
    match kept {
        Cell::File(range) => Cell::File(piece(range)),
        Cell::Text(range) => Cell::Text(piece(range)),
        Cell::Number(number) => Cell::Number(number),
        Cell::Span(span) => Cell::Span(span),
    }
}

/// The cell `lent` is, owning its text.
#[must_use]
pub fn owned(lent: Cell<CellText<'_>>) -> Cell {
    match lent {
        Cell::File(path) => Cell::File(path.0.into()),
        Cell::Text(text) => Cell::Text(text.0.into()),
        Cell::Number(number) => Cell::Number(number),
        Cell::Span(span) => Cell::Span(span),
    }
}

/// Keeps the text of `cell` at the end of `text`, the text of its rows, and answers the cell as kept there.
pub fn keep(text: &mut RenderedText, cell: Cell<CellText<'_>>) -> Cell<TextRange> {
    let mut put = |piece: CellText<'_>| {
        let start = RowsTextAt(u32::try_from(text.0.len()).unwrap_or(u32::MAX));
        text.0.push_str(piece.0);
        TextRange {
            start,
            end: RowsTextAt(u32::try_from(text.0.len()).unwrap_or(u32::MAX)),
        }
    };
    match cell {
        Cell::File(path) => Cell::File(put(path)),
        Cell::Text(piece) => Cell::Text(put(piece)),
        Cell::Number(number) => Cell::Number(number),
        Cell::Span(span) => Cell::Span(span),
    }
}
