//! The filler of `source-location`: where every element of a walked file is, by its lines, its columns and its bytes. Today's
//! reading answers the relation with the engine's own reader of an element table; the walk's table has no such reader, so the
//! language fills it here, beside its walk.

use crate::domain::file_view::service::{node_cell, span_at};
use crate::domain::file_view::types::TypeScriptWalk;
use ryo::languages::{self, Integer, Position, node_table};

/// Fills the `source-location` rows of one TypeScript file from its node table.
pub(crate) struct SourceLocationItem;

/// The offsets at which the lines of the text start: a carriage return and a line feed are one break, and each alone is one too.
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

/// The one-based line and the zero-based byte column of an offset.
fn place(starts: &[u32], offset: u32) -> (Integer, Integer) {
    let line = starts.partition_point(|start| *start <= offset).max(1);
    let start = starts.get(line - 1).copied().unwrap_or(0);
    (Integer(i64::try_from(line).unwrap_or(i64::MAX)), Integer(i64::from(offset.saturating_sub(start))))
}

ryo::fills! {
    SourceLocationItem => languages::SourceLocation, walk(TypeScriptWalk) |file, _words, rows| {
        /// One row of the relation.
        type Row = <languages::SourceLocation as languages::CatalogRelation>::Row;
        let starts = line_starts(file.source.0);
        let count = u32::try_from(node_table::element_count(file.table).0).unwrap_or(u32::MAX);
        for id in (0..count).map(Position) {
            let span = span_at(file, id);
            if span.start > span.end || usize::try_from(span.end.0).unwrap_or(usize::MAX) > file.source.0.len() {
                continue;
            }
            let ((start_line, start_column), (end_line, end_column)) = (place(&starts, span.start.0), place(&starts, span.end.0));
            rows.write(Row {
                id: node_cell(file, id),
                file: file.file,
                span,
                start_line,
                end_line,
                start_byte: Integer(i64::from(span.start.0)),
                end_byte: Integer(i64::from(span.end.0)),
                start_column,
                end_column,
            });
        }
    }
}
