//! Today's reading of a source, for the tests and the row-equality bench of the language's walk: its table and its rows in the
//! form the walk's are compared in, and the sources of the language's fixtures.

use super::walking::{Keys, Placement, Seen, column_kinds, element, keyed, plain, special};
use ryo::languages::{self, ByteSlice, CellKind, FileId, Offset, Position, Relation, contract};

/// The key of each element of the file's own tree in today's table: the embedded expressions after it are left out.
#[must_use]
pub fn read_keys(elements: &languages::Elements) -> Keys {
    let end = if languages::end_of(elements).0 == 0 {
        0
    } else {
        languages::last_at(elements, Position(0)).0 + 1
    };
    keyed((0..end).map(|at| {
        (
            languages::kind_at(elements, Position(at)),
            languages::start_at(elements, Position(at)),
            languages::end_at(elements, Position(at)),
        )
    }))
}

/// Where each element of today's table stands, sorted. The table must have been read with its fields wanted.
#[must_use]
pub fn read_placements(elements: &languages::Elements, keys: &Keys) -> Vec<Placement> {
    let mut placements: Vec<Placement> = keys
        .0
        .iter()
        .enumerate()
        .map(|(index, element)| {
            let at = Position(u32::try_from(index).unwrap_or(u32::MAX));
            let parent = languages::parent_at(elements, at);
            let field = languages::field_at(elements, at);
            Placement {
                element: *element,
                parent: keys.0.get(usize::from(parent)).copied().unwrap_or(*element),
                ordinal: languages::ordinal_at(elements, at).0,
                field: if field == languages::NO_FIELD {
                    0
                } else {
                    field.0
                },
            }
        })
        .collect();
    placements.sort_unstable();
    placements
}

/// Today's reading of one file: its table, its names and every row it wrote.
#[derive(Debug, Default)]
pub struct Read {
    /// The table.
    pub elements: languages::Elements,
    /// The names.
    pub symbols: languages::Symbols,
    /// The rows.
    pub rows: contract::Collect,
}

/// Reads `bytes` with today's reading of the language, its fields wanted and every relation in `wanted` asked.
///
/// # Errors
///
/// When the language refuses the bytes.
pub fn read(
    engine: &dyn contract::ParseEngine,
    path: &str,
    bytes: &[u8],
    wanted: &[Relation],
    into: &mut Read,
) -> Result<(), languages::ParseError> {
    into.rows.clear();
    contract::read(
        engine,
        languages::ParseRequest {
            path: languages::SourcePath(path),
            file: FileId(0),
            bytes: ByteSlice(bytes),
            notes: languages::NoteRequest::Unwanted,
            fields: languages::FieldRequest::Wanted,
        },
        &mut into.elements,
        &mut into.symbols,
        wanted,
        &mut into.rows,
    )
}

/// The rows of `relation` today's reading wrote, sorted.
#[must_use]
pub fn read_rows(read: &Read, bytes: &[u8], keys: &Keys, relation: Relation) -> Vec<Vec<Seen>> {
    let kinds = column_kinds(relation);
    let mut rows: Vec<Vec<Seen>> = read
        .rows
        .rows()
        .into_iter()
        .filter(|(each, _)| *each == relation)
        .map(|(_, words)| {
            kinds
                .iter()
                .zip(&words)
                .map(|(kind, word)| match kind {
                    CellKind::Node => element(keys, *word),
                    CellKind::Symbol => special(*word).unwrap_or_else(|| {
                        let symbol = languages::Symbol(u32::try_from(*word).unwrap_or(u32::MAX));
                        Seen::Text(
                            languages::text_of(&read.symbols, ByteSlice(bytes), symbol)
                                .unwrap_or_default()
                                .0
                                .to_owned(),
                        )
                    }),
                    CellKind::File | CellKind::Span | CellKind::Number | CellKind::Flag => {
                        plain(*kind, *word)
                    }
                })
                .collect()
        })
        .collect();
    rows.sort_unstable();
    rows
}

/// The offset of a point of a text: its line and its column, both from one, the column in bytes.
fn offset_of(lines: &[usize], point: contract::Point) -> Offset {
    let line = usize::try_from(point.line)
        .unwrap_or(usize::MAX)
        .saturating_sub(1);
    let start = lines.get(line).copied().unwrap_or_default();
    let column = usize::try_from(point.column)
        .unwrap_or(usize::MAX)
        .saturating_sub(1);
    Offset(u32::try_from(start + column).unwrap_or(u32::MAX))
}

/// The rows a fixture states for `bytes`, in the form the walk's rows are compared in, sorted: a node is the element at its place
/// in today's table, a span its offsets, an unknown value the number of its reason.
#[must_use]
pub fn stated_rows(
    expected: &contract::Expected,
    bytes: &[u8],
    keys: &Keys,
    reasons: &[languages::UnknownName],
) -> Vec<Vec<Seen>> {
    let mut lines = vec![0_usize];
    lines.extend(
        bytes
            .iter()
            .enumerate()
            .filter(|(_, byte)| **byte == b'\n')
            .map(|(at, _)| at + 1),
    );
    let mut rows: Vec<Vec<Seen>> = expected
        .rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|cell| match cell {
                    contract::Cell::Unlocated => {
                        Seen::Span(languages::NO_SPAN.start, languages::NO_SPAN.end)
                    }
                    contract::Cell::Node(place) => element(keys, u64::from(*place)),
                    contract::Cell::Name(text) => Seen::Text(text.clone()),
                    contract::Cell::Unknown(name) => {
                        reasons.iter().find(|each| each.text.0 == name).map_or_else(
                            || Seen::Text(name.clone()),
                            |each| Seen::Unknown(each.reason.0),
                        )
                    }
                    contract::Cell::File(_) => Seen::File,
                    contract::Cell::Span(from, to) => {
                        Seen::Span(offset_of(&lines, *from), offset_of(&lines, *to))
                    }
                    contract::Cell::Number(number) => Seen::Number(number.0),
                    contract::Cell::Flag(flag) => Seen::Flag(*flag),
                })
                .collect()
        })
        .collect();
    rows.sort_unstable();
    rows
}

/// A row as a fixture can state it: a text cell that holds no text is the empty name there.
#[must_use]
pub fn as_stated(mut row: Vec<Seen>) -> Vec<Seen> {
    for cell in &mut row {
        if *cell == Seen::NoText {
            *cell = Seen::Text(String::new());
        }
    }
    row
}
