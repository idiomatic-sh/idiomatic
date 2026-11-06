//! The `value_cells` service: free functions named by verb that make the typed cell a value is in a row of the answer, with no IO
//! and no ports. A name's text is kept here, as the row that holds it is made; a number and a span stay what they are.

use std::fmt::Write as _;

use ryo_language::domain::symbols::service::{file_in, place_of, places_of, text_of};
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_relations::domain::relations::service::{node_file, node_position};
use ryo_relations::domain::relations::types::{Name, Origin, Value};
use ryo_relations::domain::terms::service::unknown_reason;
use ryo_relations::domain::terms::types::Symbol;
use ryo_values::domain::records::service::keep;
use ryo_values::domain::row_texts::types::{Cell, CellText, RowsTextAt, TextRange};

use crate::domain::value_cells::constants::{UNKNOWN_NAMED, UNKNOWN_SHOWN};
use crate::domain::value_cells::types::Scene;
use ryo_relations::domain::terms::types::UnknownName;
use ryo_values::domain::row_texts::types::RenderedText;

/// The bytes of a name read from the file the scene is of; none for a name of another file.
fn name_bytes(name: Name, scene: Scene<'_>) -> Option<ByteSlice<'_>> {
    let place = place_of(name, places_of(scene.symbols), scene.symbols)?;
    let (start, end) = (
        usize::try_from(place.start.0).unwrap_or(usize::MAX),
        usize::try_from(place.end.0).unwrap_or(usize::MAX),
    );
    if place.origin == Origin::Source && place.file == scene.file {
        scene.source.0.get(start..end).map(ByteSlice)
    } else {
        None
    }
}

/// Keeps what `written` writes at the end of `text`, as the text of a text cell.
fn written_text(
    text: &mut RenderedText,
    written: impl FnOnce(&mut RenderedText),
) -> Cell<TextRange> {
    let start = RowsTextAt(u32::try_from(text.len()).unwrap_or(u32::MAX));
    written(&mut *text);
    Cell::Text(TextRange {
        start,
        end: RowsTextAt(u32::try_from(text.len()).unwrap_or(u32::MAX)),
    })
}

/// The cell an unknown value is: `unknown: ` and the name of its reason when one of the languages linked declares it, else `unknown`.
#[must_use]
pub(crate) fn unknown_cell(
    text: &mut RenderedText,
    symbol: Symbol,
    declared: &[UnknownName],
) -> Cell<TextRange> {
    let named = unknown_reason(symbol)
        .and_then(|reason| declared.iter().find(|each| each.reason == reason));
    written_text(text, |text| match named {
        Some(name) => write!(text, "{UNKNOWN_NAMED}{}", name.text.0).unwrap_or_default(),
        None => text.push_str(UNKNOWN_SHOWN),
    })
}

/// The cell `value` is in a row of the answer, its text kept at the end of `text`; none for a value that reads as nothing here: a
/// name of another file, or a file the run does not list. A node and a yes or no have no cell of their own and are the text they
/// read as: `path#position` and `true` or `false`.
pub(crate) fn value_cell(
    text: &mut RenderedText,
    value: Value,
    scene: Scene<'_>,
) -> Option<Cell<TextRange>> {
    let path = |file| file_in(scene.paths, file).map(|path| &*path.0);
    // Writing to the text never fails: its `write_str` only pushes.
    match value {
        Value::Node(node) => Some(written_text(text, |text| {
            // A node has no cell of its own yet: its cell is the text it reads as, `path#place`.
            write!(
                text,
                "{}#{}",
                path(node_file(node)).unwrap_or_default(),
                node_position(node).0
            )
            .unwrap_or_default();
        })),
        Value::Symbol(symbol) if unknown_reason(symbol).is_some() => {
            Some(unknown_cell(text, symbol, scene.unknown))
        }
        Value::Symbol(symbol) => text_of(scene.symbols, ByteSlice(scene.source.0), symbol)
            .map(|name| keep(text, Cell::Text(CellText(name.0)))),
        Value::Name(name) => name_bytes(name, scene).map(|bytes| {
            keep(
                text,
                Cell::Text(CellText(&String::from_utf8_lossy(bytes.0))),
            )
        }),
        Value::File(file) => path(file).map(|path| keep(text, Cell::File(CellText(path)))),
        Value::Span(span) => Some(Cell::Span(span)),
        Value::Number(number) => Some(Cell::Number(number)),
        Value::Flag(flag) => Some(written_text(text, |text| {
            write!(text, "{flag}").unwrap_or_default();
        })),
    }
}
