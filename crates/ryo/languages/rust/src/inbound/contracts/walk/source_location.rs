//! The filler of `source-location`: where every element of a walked file is, by its lines, its columns and its bytes. A line ends at
//! `\n`, at `\r` and at `\r\n`; a column is the bytes from the start of its line; a line is numbered from one.

use crate::domain::file_view::service::{split, visited};
use crate::domain::file_view::types::RustWalk;
use ryo::languages::{self, Integer, Span};

/// Fills the `source-location` rows of one Rust file from its elements.
pub(crate) struct SourceLocationItem;

/// Where each line of the bytes starts.
fn line_starts(bytes: &[u8]) -> Vec<u32> {
    let mut starts = vec![0];
    let mut at = 0;
    while let Some(byte) = bytes.get(at) {
        let newline = matches!(*byte, b'\r' | b'\n');
        if *byte == b'\r' && bytes.get(at + 1) == Some(&b'\n') {
            at += 1;
        }
        at += 1;
        if newline {
            starts.push(u32::try_from(at).unwrap_or(u32::MAX));
        }
    }
    starts
}

/// The line, from one, and the column of the offset.
fn place(starts: &[u32], offset: u32) -> (i64, i64) {
    let through = starts.partition_point(|start| *start <= offset);
    let start = through.checked_sub(1).and_then(|line| starts.get(line)).copied().unwrap_or(0);
    (i64::try_from(through).unwrap_or(i64::MAX), i64::from(offset.saturating_sub(start)))
}

ryo::fills! {
    SourceLocationItem => languages::SourceLocation, walk(RustWalk) |file, _words, rows| {
        /// One row of the relation.
        type Row = <languages::SourceLocation as languages::CatalogRelation>::Row;
        let (owned, _symbols, _model, _table) = split(file);
        let tree = &owned;
        let starts = line_starts(tree.source.0);
        for position in visited(tree.elements, true) {
            let span = Span {
                start: languages::start_at(tree.elements, position),
                end: languages::end_at(tree.elements, position),
            };
            if span.start > span.end || usize::try_from(span.end.0).unwrap_or(usize::MAX) > tree.source.0.len() {
                continue;
            }
            let ((start_line, start_column), (end_line, end_column)) = (place(&starts, span.start.0), place(&starts, span.end.0));
            rows.write(Row {
                id: languages::node_id(tree.file, position),
                file: tree.file,
                span,
                start_line: Integer(start_line),
                end_line: Integer(end_line),
                start_byte: Integer(i64::from(span.start.0)),
                end_byte: Integer(i64::from(span.end.0)),
                start_column: Integer(start_column),
                end_column: Integer(end_column),
            });
        }
    }
}
