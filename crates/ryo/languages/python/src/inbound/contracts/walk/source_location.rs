//! The filler of `source-location`: where every element of a walked file is, by its lines, its columns and its bytes. Today's
//! reading answers the relation with the engine's own reader of an element table; the walk's table has no such reader, so the
//! language fills it here, beside its walk.

use crate::domain::file_view::types::PythonWalk;
use crate::domain::layout_facts::service::{line_place, line_starts};
use crate::domain::node::service::{node_cell, span_at};
use ryo::languages::{self, Integer, Position, node_table};

/// Fills the `source-location` rows of one Python file from its node table.
pub(crate) struct SourceLocationItem;

ryo::fills! {
    SourceLocationItem => languages::SourceLocation, walk(PythonWalk) |file, _words, rows| {
        /// One row of the relation.
        type Row = <languages::SourceLocation as languages::CatalogRelation>::Row;
        let lines = &mut file.buffers.lines;
        line_starts(&file.syntax, lines);
        let count = u32::try_from(node_table::element_count(file.syntax.table).0).unwrap_or(u32::MAX);
        for id in (0..count).map(Position) {
            let span = span_at(&file.syntax, id);
            let (start, end) = (line_place(lines, span.start), line_place(lines, span.end));
            rows.write(Row {
                id: node_cell(&file.syntax, id),
                file: file.syntax.file,
                span,
                start_line: start.line,
                end_line: end.line,
                start_byte: Integer(i64::from(span.start.0)),
                end_byte: Integer(i64::from(span.end.0)),
                start_column: start.column,
                end_column: end.column,
            });
        }
    }
}
