//! The `finish` service: free functions named by verb that turn the rows a query emits into
//! findings, with no IO and no ports. A finding is numbers until a report is written.

use crate::domain::table_texts::types::SourceView;
use ryo_language::domain::relations::service::{node_file, node_position};
use ryo_language::domain::relations::types::Bytes;
use ryo_language::domain::relations::types::Integer;
use ryo_language::domain::relations::types::{
    CROSS_FILE, FileId, Name, NamePlace, NodeId, Offset, Origin, Value, WorkerNumber,
};
use ryo_language::domain::symbols::service::derived_of;
use ryo_language::domain::symbols::service::places_of;
use ryo_language::domain::symbols::service::{
    file_in, intern_derived, name_made_across, name_of, place_of, text_of,
};
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_language::domain::symbols::types::Symbols;
use ryo_language::domain::terms::service::unknown_reason;

use super::errors::FinishError;
use super::types::{
    Arena, ColumnNumber, FileLine, FoundRow, Leaves, Leaving, LineStarts, Loaded, Point, Resolving,
    Retained, RuleNumber,
};
use crate::domain::table_model::service::{cell, column_in, positions, row_in};
use crate::domain::table_model::types::Table;
use crate::domain::value_cells::service::unknown_cell;
use ryo_values::domain::plans::types::Emit;
use ryo_values::domain::records::service::keep;
use ryo_values::domain::records::types::RenderedText;
use ryo_values::domain::records::types::{Cell, CellText, TextRange};

/// Turns each name of the rows into a name that leaves its file, in place: its text stays where
/// it is, and the row carries the hash of the text and the number of its place, which is pushed onto `places`.
pub(crate) fn escape(table: &mut Table, leaving: Leaving<'_>, kept: Retained<'_>) {
    let Leaving {
        symbols,
        source,
        file,
        worker,
        text,
    } = leaving;
    let Retained { arena, places } = kept;
    for value in &mut table.cells {
        if let Value::Symbol(symbol) = value {
            let mut made = Bytes(std::mem::take(&mut *arena.0));
            let named = name_of(
                symbols,
                ByteSlice(source.0),
                *symbol,
                file,
                worker,
                &mut made,
                places,
            );
            *arena.0 = made.0;
            let Some(name) = named else { continue };
            if text == Leaves::WithText
                && let Some(place) = places.last_mut()
            {
                *place = in_arena(*place, source, worker, Arena(&mut *arena.0));
            }
            *value = Value::Name(name);
        }
    }
}

/// Turns each name of a table the project wrote (its text is in `from`) into a name of the steps that run after
/// the files were read, whose text is in `cross`: it compares with the names the files handed over.
pub(crate) fn carry(table: &mut Table, from: &Symbols, cross: &mut Symbols) {
    for value in &mut table.cells {
        if let Value::Symbol(symbol) = value
            && let Some(text) = text_of(from, ByteSlice(&[]), *symbol)
        {
            let made = intern_derived(cross, ByteSlice(&[]), text);
            if let Some(name) = name_made_across(cross, made) {
                *value = Value::Name(name);
            }
        }
    }
}

/// A place whose text is in the file, made one whose text is in the worker's arena: steps that read the text
/// after the files were read cannot reach the file.
fn in_arena(
    place: NamePlace,
    source: SourceView<'_>,
    worker: WorkerNumber,
    arena: Arena<'_>,
) -> NamePlace {
    let arena = arena.0;
    if place.origin != Origin::Source {
        return place;
    }
    let text = source
        .0
        .get(
            usize::try_from(place.start.0).unwrap_or(usize::MAX)
                ..usize::try_from(place.end.0).unwrap_or(usize::MAX),
        )
        .unwrap_or_default();
    let start = u32::try_from(arena.len()).unwrap_or(u32::MAX);
    arena.extend_from_slice(text);
    NamePlace {
        origin: Origin::Derived(worker),
        start: Offset(start),
        end: Offset(u32::try_from(arena.len()).unwrap_or(u32::MAX)),
        ..place
    }
}

/// One finding for each row of `rows`, none placed yet: it is of `file`, unless the row's subject
/// is a file, which it is then.
///
/// # Errors
///
/// `NoSuchColumn` when the rows lack the subject column.
pub(crate) fn founds(
    rule: RuleNumber,
    rows: &Table,
    emit: &Emit,
    file: FileId,
) -> Result<Vec<FoundRow>, FinishError> {
    let mut found = Vec::new();
    for index in positions(rows.count) {
        let row = row_in(rows, index).unwrap_or_default();
        found.push(FoundRow {
            rule,
            file: subject_file(row, emit, file)?,
            place: Point {
                line: FileLine(0),
                column: ColumnNumber(0),
            },
            row: row.to_vec(),
        });
    }
    Ok(found)
}

/// The file a row is a finding of: the one its subject column names, or `file`.
pub(crate) fn subject_file(
    row: &[Value],
    emit: &Emit,
    file: FileId,
) -> Result<FileId, FinishError> {
    Ok(match cell(row, emit.subject)? {
        Value::File(subject) => subject,
        Value::Node(_)
        | Value::Symbol(_)
        | Value::Name(_)
        | Value::Span(_)
        | Value::Number(Integer(_))
        | Value::Flag(_) => file,
    })
}

/// Where in its file a row is: the start of its first key column that is a stretch, a node or a
/// name, or of its subject.
pub(crate) fn offset_of(
    row: &[Value],
    emit: &Emit,
    nodes: &dyn Fn(NodeId) -> Option<Offset>,
    names: &dyn Fn(Name) -> Option<NamePlace>,
) -> Option<Offset> {
    emit.key
        .iter()
        .chain(std::iter::once(&emit.subject))
        .find_map(|index| match column_in(row, *index).copied()? {
            Value::Span(span) => (ryo_language::domain::relations::service::location(span)
                == ryo_language::domain::relations::types::Locatedness::Located)
                .then_some(span.start),
            Value::Node(node) => nodes(node),
            Value::Name(name) => names(name)
                .filter(|place| matches!(place.origin, Origin::Source))
                .map(|place| place.start),
            Value::Symbol(_) | Value::File(_) | Value::Number(Integer(_)) | Value::Flag(_) => None,
        })
}

/// Where the lines of `source` start, as far as `upto`.
#[must_use]
pub(crate) fn line_starts(source: SourceView<'_>, upto: Offset) -> LineStarts {
    let mut starts = LineStarts(Vec::new());
    fill_line_starts(source, upto, &mut starts);
    starts
}

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

/// Places every finding that has an offset, reading the line starts of `source` once.
pub(crate) fn place_all(
    found: &mut [FoundRow],
    emit: &Emit,
    source: SourceView<'_>,
    across: &Symbols,
) {
    let names = |name: Name| place_of(name, places_of(across), across);
    let offsets: Vec<Option<Offset>> = found
        .iter()
        .map(|each| offset_of(&each.row, emit, &|_| None, &names))
        .collect();
    let Some(last) = offsets.iter().flatten().max() else {
        return;
    };
    let starts = line_starts(source, *last);
    for (each, offset) in found.iter_mut().zip(offsets) {
        if let Some(offset) = offset {
            each.place = place_in(source, &starts, offset);
        }
    }
}

/// The bytes of a name that left its file: from the worker's text, or from the file it was read from; none when they are nowhere.
fn name_bytes<'a>(
    name: Name,
    resolving: &Resolving<'a>,
    loaded: &'a Loaded,
) -> Option<ByteSlice<'a>> {
    let place = place_of(name, places_of(resolving.cross), resolving.cross)?;
    let (start, end) = (
        usize::try_from(place.start.0).unwrap_or(usize::MAX),
        usize::try_from(place.end.0).unwrap_or(usize::MAX),
    );
    match place.origin {
        Origin::Derived(worker) if worker == CROSS_FILE => {
            derived_of(resolving.cross).0.get(start..end)
        }
        Origin::Derived(worker) => resolving
            .arenas
            .0
            .get(usize::try_from(worker.0).unwrap_or(usize::MAX))
            .and_then(|arena| arena.0.get(start..end)),
        Origin::Source => loaded.bytes.0.get(start..end),
    }
    .map(ByteSlice)
}

/// The cell a value is in a report, its text kept at the end of `text`; none for a value that reads as nothing: a name whose text
/// is nowhere, or a file the run does not list. A node and a yes or no have no cell of their own and are the text they read as.
#[must_use]
pub(crate) fn shown_cell(
    text: &mut RenderedText,
    value: Value,
    resolving: &Resolving<'_>,
    loaded: &Loaded,
) -> Option<Cell<TextRange>> {
    let path = |file: FileId| file_in(resolving.paths, file).map(|path| &*path.0);
    match value {
        Value::Node(node) => {
            let shown = format!(
                "{}#{}",
                path(node_file(node)).unwrap_or_default(),
                node_position(node).0
            );
            Some(keep(text, Cell::Text(CellText(&shown))))
        }
        Value::Symbol(symbol) if unknown_reason(symbol).is_some() => {
            Some(unknown_cell(text, symbol, resolving.unknown))
        }
        Value::Symbol(symbol) => text_of(resolving.cross, ByteSlice(&[]), symbol)
            .map(|name| keep(text, Cell::Text(CellText(name.0)))),
        Value::Name(name) => name_bytes(name, resolving, loaded).map(|bytes| {
            keep(
                text,
                Cell::Text(CellText(&String::from_utf8_lossy(bytes.0))),
            )
        }),
        Value::File(file) => path(file).map(|path| keep(text, Cell::File(CellText(path)))),
        Value::Span(span) => Some(Cell::Span(span)),
        Value::Number(number) => Some(Cell::Number(number)),
        Value::Flag(flag) => Some(keep(text, Cell::Text(CellText(&flag.to_string())))),
    }
}

/// The cell of the line of a place: none for a row with no place in its file.
#[must_use]
pub(crate) fn line_cell<T>(place: Point) -> Option<Cell<T>> {
    (place.line.0 != 0).then_some(Cell::Number(Integer(i64::from(place.line.0))))
}

/// The cell of the character of a place: none for a row with no place in its file.
#[must_use]
pub(crate) fn column_cell<T>(place: Point) -> Option<Cell<T>> {
    (place.column.0 != 0).then_some(Cell::Number(Integer(i64::from(place.column.0))))
}

/// The source a value needs before it can be rendered, if its spelling was retained by location.
#[must_use]
pub(crate) fn source_of(value: Value, resolving: &Resolving<'_>) -> Option<FileId> {
    let Value::Name(name) = value else {
        return None;
    };
    place_of(name, places_of(resolving.cross), resolving.cross)
        .filter(|place| place.origin == Origin::Source)
        .map(|place| place.file)
}
