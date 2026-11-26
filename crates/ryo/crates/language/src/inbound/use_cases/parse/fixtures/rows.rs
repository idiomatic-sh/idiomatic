//! The rows a language answers for a fixture, read as cells to be held against what the fixture expects.

use crate::inbound::use_cases::parse::collect::Collect;
use super::expected::{Cell, Point};
use super::findings::Findings;
use crate::domain::names::service::{derived_of, places_of};
use crate::domain::symbols::service::{place_of, text_of};
use crate::domain::symbols::types::{ByteSlice, Symbols};
use ryo_relations::domain::relations::constants::NO_FILE;
use ryo_relations::domain::relations::service::from_bits;
use ryo_relations::domain::relations::service::{kinds, node_position};
use ryo_relations::domain::relations::types::{Bits, Flag, Name, Origin, Relation, Value};
use ryo_relations::domain::terms::service::unknown_reason;
use ryo_relations::domain::terms::types::UnknownName;

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
            if ryo_relations::domain::relations::service::location(span)
                == ryo_relations::domain::relations::types::Locatedness::Located
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

/// Independent inputs may share local node numbers only when every row names its file.
pub(super) fn check_input_identity(
    count: usize,
    kinds: &[ryo_relations::domain::relations::types::CellKind],
    folder: &str,
    findings: &mut Findings,
) -> bool {
    if count <= 1 || kinds.contains(&ryo_relations::domain::relations::types::CellKind::File) {
        return true;
    }
    findings.0.push(format!(
        "{folder}: multiple inputs require a File column for unambiguous row identity"
    ));
    false
}

/// Holds `got` to `expected`, row by row, and says what differs. A node is named by its place in its file's table, and the numbering of
/// a table is the walk's own choice: the rows are the same rows when a one-to-one renaming of the nodes makes them equal (what the
/// frozen digests hold too, which name a node by its kind and its extent). A row that no renaming makes equal is reported with both
/// rows.
pub(super) fn compare(
    what: &str,
    got: &[Vec<Cell>],
    expected: &[Vec<Cell>],
    show: &dyn Fn(&[Cell]) -> String,
    findings: &mut Findings,
) {
    // Each file of a fixture numbers its own nodes: the renaming is made for one file at a time (the rows name their file).
    let file_of = |row: &Vec<Cell>| row.iter().find_map(|cell| if let Cell::File(path) = cell { Some(path.clone()) } else { None });
    let mut files: Vec<Option<String>> = got.iter().chain(expected).map(&file_of).collect();
    files.sort();
    files.dedup();
    for file in files {
        let got_rows: Vec<&Vec<Cell>> = got.iter().filter(|row| file_of(row) == file).collect();
        let expected_rows: Vec<&Vec<Cell>> = expected.iter().filter(|row| file_of(row) == file).collect();
        let (got_owned, expected_owned): (Vec<Vec<Cell>>, Vec<Vec<Cell>>) = (got_rows.into_iter().cloned().collect(), expected_rows.into_iter().cloned().collect());
        let (_, unmatched_got, unmatched_expected) = pair_up_to_nodes(&got_owned, &expected_owned);
        for row in unmatched_got {
            findings.0.push(format!("{what}: wrote a row not in the expected rows: {}", show(&got_owned[row])));
        }
        for row in unmatched_expected {
            findings.0.push(format!("{what}: expected a row it did not write: {}", show(&expected_owned[row])));
        }
    }
}

/// The cells of a row with its nodes taken out (their places kept apart).
fn split_nodes(row: &[Cell]) -> (Vec<&Cell>, Vec<(usize, u32)>) {
    let mut rest = Vec::new();
    let mut nodes = Vec::new();
    for (at, cell) in row.iter().enumerate() {
        match cell {
            Cell::Node(place) => nodes.push((at, *place)),
            other => rest.push(other),
        }
    }
    (rest, nodes)
}

/// Pairs each written row with an expected row that a one-to-one renaming of the nodes makes the same, and gives the rows left over on
/// each side.
fn pair_up_to_nodes(got: &[Vec<Cell>], expected: &[Vec<Cell>]) -> (usize, Vec<usize>, Vec<usize>) {
    use std::collections::HashMap;
    let got_parts: Vec<_> = got.iter().map(|row| split_nodes(row)).collect();
    let expected_parts: Vec<_> = expected.iter().map(|row| split_nodes(row)).collect();
    let mut got_left: Vec<usize> = (0..got.len()).collect();
    let mut expected_left: Vec<usize> = (0..expected.len()).collect();
    // The renaming so far, from a written node to an expected one and back.
    let mut forward: HashMap<u32, u32> = HashMap::new();
    let mut backward: HashMap<u32, u32> = HashMap::new();
    let fits = |forward: &HashMap<u32, u32>, backward: &HashMap<u32, u32>, g: &[(usize, u32)], e: &[(usize, u32)]| -> bool {
        g.len() == e.len()
            && g.iter().zip(e).all(|((gat, gnode), (eat, enode))| {
                gat == eat && forward.get(gnode).is_none_or(|mapped| mapped == enode) && backward.get(enode).is_none_or(|mapped| mapped == gnode)
            })
    };
    let mut matched = 0;
    loop {
        let mut progress = false;
        let mut at = 0;
        while at < expected_left.len() {
            let e = expected_left[at];
            let candidates: Vec<usize> = got_left
                .iter()
                .copied()
                .filter(|&g| got_parts[g].0 == expected_parts[e].0 && fits(&forward, &backward, &got_parts[g].1, &expected_parts[e].1))
                .collect();
            if candidates.len() == 1 {
                let g = candidates[0];
                for ((_, gnode), (_, enode)) in got_parts[g].1.iter().zip(&expected_parts[e].1) {
                    let _ = forward.insert(*gnode, *enode);
                    let _ = backward.insert(*enode, *gnode);
                }
                got_left.retain(|&row| row != g);
                expected_left.remove(at);
                matched += 1;
                progress = true;
            } else {
                at += 1;
            }
        }
        if !progress {
            break;
        }
    }
    // Rows no single choice settles (the same cells more than once): the first that fits, in order.
    let mut at = 0;
    while at < expected_left.len() {
        let e = expected_left[at];
        if let Some(position) = got_left.iter().position(|&g| got_parts[g].0 == expected_parts[e].0 && fits(&forward, &backward, &got_parts[g].1, &expected_parts[e].1)) {
            let g = got_left.remove(position);
            for ((_, gnode), (_, enode)) in got_parts[g].1.iter().zip(&expected_parts[e].1) {
                let _ = forward.insert(*gnode, *enode);
                let _ = backward.insert(*enode, *gnode);
            }
            expected_left.remove(at);
            matched += 1;
        } else {
            at += 1;
        }
    }
    (matched, got_left, expected_left)
}
