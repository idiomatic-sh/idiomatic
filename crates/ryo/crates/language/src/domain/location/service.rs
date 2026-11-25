//! Source coordinates from an index built once per requested file, with no IO.

use super::types::{
    SourceByteColumn, SourceLineBytes, SourceLineIndex, SourceLineNumber, SourceLineOffset,
};

/// Builds the index only when requested. CRLF is one break; lone CR and LF also end lines.
pub(crate) fn index(index: &mut SourceLineIndex, source: SourceLineBytes<'_>) {
    let starts = &mut index.0;
    if !starts.is_empty() {
        return;
    }
    starts.push(SourceLineOffset(0));
    let mut at = 0;
    while let Some(byte) = source.0.get(at) {
        let newline = matches!(*byte, b'\r' | b'\n');
        if *byte == b'\r' && source.0.get(at + 1) == Some(&b'\n') {
            at += 1;
        }
        at += 1;
        if newline {
            starts.push(SourceLineOffset(u32::try_from(at).unwrap_or(u32::MAX)));
        }
    }
}

/// The one-based line of an endpoint, found in logarithmic time without rescanning the source.
#[cfg(test)]
#[must_use]
pub(crate) fn line(index: &SourceLineIndex, offset: SourceLineOffset) -> SourceLineNumber {
    SourceLineNumber(i64::try_from(lines_through(index, offset)).unwrap_or(i64::MAX))
}

/// Spike (ryo-fmx3, host3): how many lines start at or before `offset`, as `partition_point` answers it, tried first at the
/// line of this thread's last answer and the one after it: readers ask in source order, so the answer is mostly one of those.
fn lines_through(index: &SourceLineIndex, offset: SourceLineOffset) -> usize {
    thread_local! {
        static LAST: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    }
    let starts = &index.0;
    let fits = |through: usize| {
        through >= 1
            && through <= starts.len()
            && starts.get(through - 1).is_some_and(|start| *start <= offset)
            && starts.get(through).is_none_or(|start| *start > offset)
    };
    let last = LAST.with(std::cell::Cell::get);
    let through = if fits(last) {
        last
    } else if fits(last.saturating_add(1)) {
        last.saturating_add(1)
    } else {
        starts.partition_point(|start| *start <= offset)
    };
    LAST.with(|cell| cell.set(through));
    through
}

/// The line and the column of an endpoint from one search of the line index.
#[must_use]
pub(crate) fn place(index: &SourceLineIndex, offset: SourceLineOffset) -> (SourceLineNumber, SourceByteColumn) {
    let through = lines_through(index, offset);
    let start = through
        .checked_sub(1)
        .and_then(|line| index.0.get(line))
        .map_or(0, |start| start.0);
    (
        SourceLineNumber(i64::try_from(through).unwrap_or(i64::MAX)),
        SourceByteColumn(i64::from(offset.0.saturating_sub(start))),
    )
}

/// The zero-based UTF-8 byte column, including exclusive EOF endpoints.
#[cfg(test)]
pub(crate) fn column(index: &SourceLineIndex, offset: SourceLineOffset) -> SourceByteColumn {
    let line = lines_through(index, offset);
    let start = line
        .checked_sub(1)
        .and_then(|line| index.0.get(line))
        .map_or(0, |start| start.0);
    SourceByteColumn(i64::from(offset.0.saturating_sub(start)))
}
