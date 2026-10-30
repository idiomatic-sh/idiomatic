//! The rows a language answers for a fixture, read as cells to be held against what the fixture expects.

use super::super::service::read;
use super::super::types::ParseEngine;
use super::super::types::{ParseRequest, Recognition};
use super::contract::Collect;
use super::expected::{Cell, Point};
use super::findings::Findings;
use crate::domain::elements::types::Elements;
use crate::domain::files::types::SourcePath;
use crate::domain::relations::constants::NO_FILE;
use crate::domain::relations::service::from_bits;
use crate::domain::relations::service::{all, kinds, node_position};
use crate::domain::relations::types::{Bits, FileId, Flag, Name, Origin, Relation, Value};
use crate::domain::symbols::service::{derived_of, place_of, places_of, text_of};
use crate::domain::symbols::types::ByteSlice;
use crate::domain::symbols::types::Symbols;
use crate::domain::terms::service::unknown_reason;
use crate::domain::terms::types::UnknownName;
use crate::inbound::use_cases::parse::types::{FieldRequest, NoteRequest};
use std::path::Path;

/// What reading one fixture gave: the rows, the names they hold and the bytes the names are in.
pub(super) struct Read {
    /// The rows the read wrote, by relation.
    pub out: Collect,
    /// The names the rows hold.
    pub symbols: Symbols,
    /// The bytes of the file the names are in.
    pub bytes: Vec<u8>,
    /// The path of each file, by its number, where the rows are of a project's files; none where they are of one file.
    pub files: Vec<String>,
    /// The bytes of each file, by its number, which the spans are read as lines and columns in.
    pub texts: Vec<Vec<u8>>,
    /// The reasons the language declares for a value it cannot resolve, by their place in its list.
    pub unknowns: &'static [UnknownName],
}

/// Reads `input` as the file its folder names, asking for every relation, and answers the rows.
pub(super) fn rows_of(
    parse: &dyn ParseEngine,
    input: &Path,
    shown: &str,
    findings: &mut Findings,
) -> Option<Read> {
    let bytes = std::fs::read(input).ok()?;
    let request = ParseRequest {
        path: SourcePath(shown),
        file: FileId(0),
        bytes: ByteSlice(&bytes),
        notes: NoteRequest::Unwanted,
        fields: FieldRequest::Wanted,
    };
    if parse.recognition(&request) == Recognition::Declines {
        findings.0.push(format!(
            "{shown}: a fixture the language does not recognize: give it a path the language reads"
        ));
        return None;
    }
    let mut out = Collect::default();
    let mut symbols = Symbols::default();
    match read(
        parse,
        request,
        &mut Elements::default(),
        &mut symbols,
        all(),
        &mut out,
    ) {
        Ok(()) => Some(Read {
            out,
            symbols,
            texts: vec![bytes.clone()],
            bytes,
            files: Vec::new(),
            unknowns: parse.unknown_reasons(),
        }),
        Err(error) => {
            findings.0.push(format!(
                "{shown}: a fixture the language fails to read: {error:?}"
            ));
            None
        }
    }
}

/// The text of a name that left its file, escaped as every text is: where the language wrote it, in the bytes of the file or in
/// the text made after, or nothing when the name's text is in no place this read knows.
pub(super) fn name_text(name: Name, read: &Read) -> String {
    let Some(place) = place_of(name, places_of(&read.symbols), &read.symbols) else {
        return String::new();
    };
    let (start, end) = (
        usize::try_from(place.start.0).unwrap_or(usize::MAX),
        usize::try_from(place.end.0).unwrap_or(usize::MAX),
    );
    let holder: &[u8] = match place.origin {
        Origin::Source => &read.bytes,
        Origin::Derived(_) => &derived_of(&read.symbols).0,
    };
    holder
        .get(start..end)
        .map(|bytes| String::from_utf8_lossy(bytes).into_owned())
        .unwrap_or_default()
}

/// Where `offset` is in `text`: the line and the column, both from one, the column in bytes.
#[must_use]
pub fn point_of(text: &[u8], offset: u32) -> Point {
    let before = text
        .get(..usize::try_from(offset).unwrap_or(usize::MAX))
        .unwrap_or(text);
    let line = before.split(|byte| *byte == b'\n').count() - 1;
    let column = before
        .iter()
        .rposition(|byte| *byte == b'\n')
        .map_or(before.len(), |last| before.len() - last - 1);
    Point {
        line: u32::try_from(line + 1).unwrap_or(u32::MAX),
        column: u32::try_from(column + 1).unwrap_or(u32::MAX),
    }
}

/// One cell of a row as a test writes it: a node as its place, a file as its shown path, a name as its text, a span as a line and a
/// column on each end, read in `text`.
pub(super) fn cell(value: Value, shown: &str, read: &Read, text: &[u8]) -> Cell {
    match value {
        Value::Node(node) => Cell::Node(node_position(node).0),
        Value::Symbol(symbol) => match unknown_reason(symbol) {
            Some(reason) => Cell::Unknown(
                read.unknowns
                    .iter()
                    .find(|name| name.reason == reason)
                    .map_or_else(
                        || format!("undeclared-reason-{}", reason.0),
                        |name| name.text.0.to_owned(),
                    ),
            ),
            None => Cell::Name(
                text_of(&read.symbols, ByteSlice(&read.bytes), symbol)
                    .unwrap_or_default()
                    .0
                    .to_owned(),
            ),
        },
        Value::Name(name) => Cell::Name(name_text(name, read)),
        Value::File(file) if file == NO_FILE => Cell::File(String::new()),
        Value::File(file) => Cell::File(
            read.files
                .get(usize::try_from(file.0).unwrap_or(usize::MAX))
                .map_or_else(|| shown.to_owned(), Clone::clone),
        ),
        Value::Span(span) => {
            if crate::domain::relations::service::location(span)
                == crate::domain::relations::types::Locatedness::Located
            {
                Cell::Span(point_of(text, span.start.0), point_of(text, span.end.0))
            } else {
                Cell::Unlocated
            }
        }
        Value::Number(number) => Cell::Number(number),
        Value::Flag(flag) => Cell::Flag(flag == Flag::Raised),
    }
}

/// The rows of `relation` that `read` holds, each as its cells. Sorted.
pub(super) fn rows_of_relation(read: &Read, relation: Relation, shown: &str) -> Vec<Vec<Cell>> {
    let kinds = kinds(relation);
    let mut rows: Vec<Vec<Cell>> = read
        .out
        .rows()
        .into_iter()
        .filter(|(each, _)| *each == relation)
        .map(|(_, words)| {
            let values: Vec<Value> = kinds
                .iter()
                .zip(&words)
                .map(|(kind, bits)| from_bits(*kind, Bits(*bits)))
                .collect();
            // A span is read in the file its row names first, else in the one file there is.
            let text: &[u8] = values
                .iter()
                .find_map(|value| match value {
                    Value::File(file) => read
                        .texts
                        .get(usize::try_from(file.0).unwrap_or(usize::MAX)),
                    Value::Node(_)
                    | Value::Symbol(_)
                    | Value::Name(_)
                    | Value::Span(_)
                    | Value::Number(_)
                    | Value::Flag(_) => None,
                })
                .or_else(|| read.texts.first())
                .map_or(&[], Vec::as_slice);
            values
                .into_iter()
                .map(|value| cell(value, shown, read, text))
                .collect()
        })
        .collect();
    rows.sort();
    rows
}

/// Holds `got` to `expected`, row by row, and says what differs.
pub(super) fn compare(
    what: &str,
    got: &[Vec<Cell>],
    expected: &[Vec<Cell>],
    show: &dyn Fn(&[Cell]) -> String,
    findings: &mut Findings,
) {
    for row in got.iter().filter(|row| !expected.contains(row)) {
        findings.0.push(format!(
            "{what}: wrote a row not in the expected rows: {}",
            show(row)
        ));
    }
    for row in expected.iter().filter(|row| !got.contains(row)) {
        findings.0.push(format!(
            "{what}: expected a row it did not write: {}",
            show(row)
        ));
    }
    if got.len() != expected.len()
        && got.iter().all(|row| expected.contains(row))
        && expected.iter().all(|row| got.contains(row))
    {
        findings.0.push(format!(
            "{what}: wrote {} rows, expected {}: a row is written twice",
            got.len(),
            expected.len()
        ));
    }
}
