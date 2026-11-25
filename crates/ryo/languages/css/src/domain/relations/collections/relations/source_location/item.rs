//! The `source_location` relation of the language: where every element of a walked file is, by its lines, its columns and its bytes.
//! Today's reading answers the relation with the engine's own reader of an element table; the walk's table has no such reader, so
//! the language fills it here.

use crate::domain::file_view::service::{node_cell, span_at};
use crate::domain::file_view::types::CssWalk;
use ryo::languages::{self, Integer, Offset, Position, node_table};

/// Fills the `source_location` rows of one CSS file from its node table.
pub(crate) struct SourceLocationItem;

/// Where each line of the file starts: the first at zero, every other after a line feed or a lone carriage return.
fn line_starts(text: &[u8]) -> Vec<Offset> {
    let mut starts = vec![Offset(0)];
    starts.extend(text.iter().enumerate().filter_map(|(at, &byte)| {
        let next = at + 1;
        let newline = byte == b'\n' || (byte == b'\r' && text.get(next) != Some(&b'\n'));
        newline.then(|| Offset(u32::try_from(next).unwrap_or(u32::MAX)))
    }));
    starts
}

/// The line, from one, and the column, in bytes from the start of the line, of an offset of the file.
fn place(starts: &[Offset], offset: Offset) -> (Integer, Integer) {
    let line = starts.partition_point(|start| *start <= offset).max(1);
    let start = starts.get(line - 1).copied().unwrap_or(Offset(0));
    (
        Integer(i64::try_from(line).unwrap_or(i64::MAX)),
        Integer(i64::from(offset.0.saturating_sub(start.0))),
    )
}

ryo::fills! {
    SourceLocationItem => languages::SourceLocation, walk(CssWalk) |file, _words, rows| {
        /// One row of the relation.
        type Row = <languages::SourceLocation as languages::CatalogRelation>::Row;
        let syntax = file.syntax;
        let starts = line_starts(syntax.text);
        let count = u32::try_from(node_table::element_count(syntax.table).0).unwrap_or(u32::MAX);
        for id in (0..count).map(Position) {
            let span = span_at(&syntax, id);
            let (start_line, start_column) = place(&starts, span.start);
            let (end_line, end_column) = place(&starts, span.end);
            rows.write(Row {
                id: node_cell(&syntax, id),
                file: syntax.file,
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
