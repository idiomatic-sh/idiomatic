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
#[must_use]
pub(crate) fn line(index: &SourceLineIndex, offset: SourceLineOffset) -> SourceLineNumber {
    SourceLineNumber(
        i64::try_from(index.0.partition_point(|start| *start <= offset)).unwrap_or(i64::MAX),
    )
}

/// The zero-based UTF-8 byte column, including exclusive EOF endpoints.
pub(crate) fn column(index: &SourceLineIndex, offset: SourceLineOffset) -> SourceByteColumn {
    let line = index.0.partition_point(|start| *start <= offset);
    let start = line
        .checked_sub(1)
        .and_then(|line| index.0.get(line))
        .map_or(0, |start| start.0);
    SourceByteColumn(i64::from(offset.0.saturating_sub(start)))
}
