//! The `finish` service: free functions named by verb that turn the rows a query emits into
//! findings, with no IO and no ports. A finding is numbers until a report is written.

use ryo_language::domain::names::service::derived_of;
use ryo_language::domain::names::service::places_of;
use ryo_language::domain::symbols::service::{file_in, place_of, text_of};
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::service::{node_file, node_position};
use ryo_relations::domain::relations::types::Integer;
use ryo_relations::domain::relations::types::{
    CROSS_FILE, FileId, Name, NamePlace, NodeId, Offset, Origin, Value,
};
use ryo_relations::domain::terms::service::unknown_reason;
use ryo_values::domain::files::types::SourceView;


use super::types::{ColumnNumber, FileLine, LineStarts, Point};

use ryo_values::domain::query_plans::types::Emit;
use ryo_values::domain::row_texts::types::RenderedText;
use ryo_values::domain::row_texts::types::{Cell, CellText, TextRange};

/// Where the lines of `source` start, as far as `upto`, written into `starts`, whose room is reused.
pub(crate) fn fill_line_starts(source: SourceView<'_>, upto: Offset, starts: &mut LineStarts) {
    let end = usize::try_from(upto.0)
        .unwrap_or(usize::MAX)
        .min(source.0.len());
    starts.0.clear();
    starts.0.push(0);
    starts.0.extend(
        memchr::memchr_iter(b'\n', source.0.get(..end).unwrap_or_default())
            .map(|at| u32::try_from(at).unwrap_or(u32::MAX).saturating_add(1)),
    );
}

/// The line and character column, counting from one, of a byte offset.
#[must_use]
pub(crate) fn place_in(source: SourceView<'_>, starts: &LineStarts, offset: Offset) -> Point {
    let line = starts
        .0
        .partition_point(|start| *start <= offset.0)
        .saturating_sub(1);
    let begins = starts
        .0
        .get(line)
        .map_or(0, |start| usize::try_from(*start).unwrap_or(0))
        .min(source.0.len());
    let at = usize::try_from(offset.0)
        .unwrap_or(usize::MAX)
        .clamp(begins, source.0.len());
    let column = String::from_utf8_lossy(source.0.get(begins..at).unwrap_or_default())
        .chars()
        .count();
    Point {
        line: FileLine(u32::try_from(line).unwrap_or(u32::MAX).saturating_add(1)),
        column: ColumnNumber(u32::try_from(column).unwrap_or(u32::MAX).saturating_add(1)),
    }
}

use ryo_values::domain::rows::types::Table;

use ryo_values::domain::runs::types::RuleNumber;
