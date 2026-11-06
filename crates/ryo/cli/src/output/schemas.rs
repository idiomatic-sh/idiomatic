// the `--json` document of rows is written as the rows come, one row at a time: its frame is the fixed bytes below, and each value inside it is written by `serde_json`; a whole-document serializer would hold every row before writing one
//! The shape of the data this output does not control: each wire shape is a `<Name>Schema`, its conversion into the core's type is an `impl TryFrom<<Name>Schema>` (or an `impl From<<Name>Schema>` where it cannot fail), and each refusal of a conversion is a `<Name>SchemaError`.

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;

use std::io::Write as _;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, PoisonError};

use ryo_core::outbound::row_sink::errors::RowSinkError;
use ryo_core::outbound::row_sink::interface::RowSinkPort;
use ryo_core::outbound::row_sink::types::{LentCell, AnswerCounts};
use ryo_relations::domain::relations::constants::UNLOCATED;
use ryo_relations::domain::relations::service::location;
use ryo_relations::domain::relations::types::Locatedness;
use ryo_values::domain::plan_parts::types::Heading;
use ryo_values::domain::records::types::{Unread, Unreadable};
use ryo_values::domain::row_texts::types::{Cell, CellText, RowTotal};

use super::Format;

/// Between the cells of a line of text.
const TAB: &[u8] = b"\t";
/// Before each line of text after the headings.
const NEWLINE: &[u8] = b"\n";
/// Before the name of a file nothing read, in text.
const UNREAD_LINE: &[u8] = b"\nunread ";
/// Opens the document.
const OBJECT_OPEN: &[u8] = b"{";
/// Ends the key of a member of the document, before its value.
const KEY_END: &[u8] = b":";
/// Between two rows, and between two cells of a row.
const COMMA: &[u8] = b",";
/// Opens a row of the document, and the list of rows.
const ROW_OPEN: &[u8] = b"[";
/// Ends a row of the document, and the rows.
const CLOSE: &[u8] = b"]";
/// Ends the document.
const DOCUMENT_CLOSE: &[u8] = b"}\n";

/// A key of the JSON document, written as `serde_json` writes its name.
#[derive(Clone, Copy, serde::Serialize)]
#[serde(rename_all = "lowercase")]
enum Key {
    /// The headings.
    Columns,
    /// The rows.
    Rows,
    /// The files nothing read.
    Unread,
    /// How many rows the answer has.
    Count,
    /// How many rows were shown.
    Shown,
}
/// Between a path and why it was not read.
const COLON: &str = ": ";
/// Why a file was not read when no linked language reads files like it.
const NO_LANGUAGE: &str = "no linked language reads this file";
/// After the rows of a cut answer, once its count has been written.
const ROWS_WORD: &[u8] = b" rows; showing ";
/// Ends the line that says how many rows were shown.
const MORE: &[u8] = b". Use --limit <n> for more.";

/// A typed value of a row as the command line prints it, the one place a cell becomes text. As text: a file and a text as they are,
/// a number in decimal, a span as its two ends (`unlocated` when the parser gave none), and a cell with no value as nothing. As
/// JSON: a file and a text as strings, a number as a number, a span as `{"start":…,"end":…}`, and a cell with no value, or a span
/// the parser gave none, as `null`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CellSchema<'a>(pub Option<Cell<CellText<'a>>>);

impl std::fmt::Display for CellSchema<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            Some(Cell::File(text) | Cell::Text(text)) => f.write_str(text.0),
            Some(Cell::Number(number)) => write!(f, "{}", number.0),
            Some(Cell::Span(span)) if location(span) == Locatedness::Absent => {
                f.write_str(UNLOCATED)
            }
            Some(Cell::Span(span)) => write!(f, "{}..{}", span.start.0, span.end.0),
            None => Ok(()),
        }
    }
}

impl serde::Serialize for CellSchema<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct as _;
        match self.0 {
            Some(Cell::File(text) | Cell::Text(text)) => serializer.serialize_str(text.0),
            Some(Cell::Number(number)) => serializer.serialize_i64(number.0),
            Some(Cell::Span(span)) if location(span) == Locatedness::Located => {
                let mut ends = serializer.serialize_struct("Span", 2)?;
                ends.serialize_field("start", &span.start.0)?;
                ends.serialize_field("end", &span.end.0)?;
                ends.end()
            }
            Some(Cell::Span(_)) | None => serializer.serialize_none(),
        }
    }
}

/// Whether the stream can still be written to.
#[derive(Debug)]
enum StreamHealth {
    /// It can.
    Well,
    /// Whoever reads it stopped reading.
    Closed,
    /// It failed, saying this.
    Failed(String),
}

/// The rows of an answer as the command line prints them, written to a stream as each row comes. As text: the headings, then a line
/// a row with a tab between its cells, then, for a cut answer, how many rows it has, then a line for each file nothing read. As JSON:
/// one document, `{"columns":[..],"rows":[[..]],"count":n,"shown":n,"unread":[..]}`.
pub(crate) struct RowsSchema {
    /// The stream the rows are written to.
    out: Mutex<Box<dyn std::io::Write + Send>>,
    /// Whether the stream can still be written to, and why not when it cannot.
    health: Mutex<StreamHealth>,
    /// Whether no row has been written yet, so the next one is the first.
    first: AtomicBool,
    /// Whether the rows are one JSON document rather than text.
    json: bool,
}

impl std::fmt::Debug for RowsSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(stringify!(RowsSchema))
            .field("json", &self.json)
            .finish_non_exhaustive()
    }
}

/// The rows of an answer, written to `out` as `format` prints them: one JSON document for JSON, text for the others.
pub(crate) fn rows(out: Box<dyn std::io::Write + Send>, format: Format) -> RowsSchema {
    RowsSchema {
        out: Mutex::new(out),
        health: Mutex::new(StreamHealth::Well),
        first: AtomicBool::new(true),
        json: format == Format::Json,
    }
}

/// The health of a stream that refused a write with `error`.
fn refused(error: &std::io::Error) -> StreamHealth {
    if error.kind() == std::io::ErrorKind::BrokenPipe {
        StreamHealth::Closed
    } else {
        StreamHealth::Failed(error.to_string())
    }
}

/// Writes the key of a member of the document and what ends it, the key written as `serde_json` writes a string.
fn key(out: &mut dyn std::io::Write, name: Key) -> std::io::Result<()> {
    serde_json::to_writer(&mut *out, &name)?;
    out.write_all(KEY_END)
}

/// A number with its thousands told apart by commas.
fn grouped(number: RowTotal) -> String {
    let digits = number.0.to_string();
    let mut text = String::with_capacity(digits.len() + digits.len() / 3);
    for (place, digit) in digits.chars().enumerate() {
        if place > 0 && (digits.len() - place).is_multiple_of(3) {
            text.push(',');
        }
        text.push(digit);
    }
    text
}

/// A file nothing read, as one line: its path, then why.
fn unread_line(file: &Unread) -> String {
    let why = match &file.reason {
        Unreadable::NoLanguage => NO_LANGUAGE,
        Unreadable::Failed(why) => &*why.0,
    };
    [&*file.path.0, COLON, why].concat()
}

impl RowsSchema {
    /// Writes what `body` writes, unless a write already failed; a write that fails marks the stream.
    fn put(&self, body: impl FnOnce(&mut dyn std::io::Write) -> std::io::Result<()>) {
        let mut out = self.out.lock().unwrap_or_else(PoisonError::into_inner);
        let mut health = self.health.lock().unwrap_or_else(PoisonError::into_inner);
        if !matches!(*health, StreamHealth::Well) {
            return;
        }
        if let Err(error) = body(&mut **out) {
            *health = refused(&error);
        }
    }
}

impl RowSinkPort for RowsSchema {
    fn begin(&self, columns: &[Heading]) {
        self.put(|out| {
            if self.json {
                let names: Vec<&str> = columns.iter().map(|heading| &*heading.0).collect();
                out.write_all(OBJECT_OPEN)?;
                key(&mut *out, Key::Columns)?;
                serde_json::to_writer(&mut *out, &names)?;
                out.write_all(COMMA)?;
                key(&mut *out, Key::Rows)?;
                out.write_all(ROW_OPEN)
            } else {
                for (place, heading) in columns.iter().enumerate() {
                    out.write_all(if place == 0 { &[] } else { TAB })?;
                    out.write_all(heading.0.as_bytes())?;
                }
                Ok(())
            }
        });
    }

    fn row(&self, cells: &mut dyn Iterator<Item = LentCell<'_>>) {
        let first = self.first.swap(false, Ordering::Relaxed);
        self.put(|out| {
            if self.json {
                out.write_all(if first { &[] } else { COMMA })?;
                out.write_all(ROW_OPEN)?;
                for (place, cell) in cells.enumerate() {
                    out.write_all(if place == 0 { &[] } else { COMMA })?;
                    serde_json::to_writer(&mut *out, &CellSchema(cell.into()))?;
                }
                out.write_all(CLOSE)
            } else {
                for (place, cell) in cells.enumerate() {
                    out.write_all(if place == 0 { NEWLINE } else { TAB })?;
                    write!(out, "{}", CellSchema(cell.into()))?;
                }
                Ok(())
            }
        });
    }

    fn end(&self, summary: AnswerCounts, unread: &[Unread]) -> Result<(), RowSinkError> {
        let lines: Vec<String> = unread.iter().map(unread_line).collect();
        self.put(|out| {
            if self.json {
                out.write_all(CLOSE)?;
                out.write_all(COMMA)?;
                key(&mut *out, Key::Count)?;
                out.write_all(summary.total.0.to_string().as_bytes())?;
                out.write_all(COMMA)?;
                key(&mut *out, Key::Shown)?;
                out.write_all(summary.shown.0.to_string().as_bytes())?;
                out.write_all(COMMA)?;
                key(&mut *out, Key::Unread)?;
                serde_json::to_writer(&mut *out, &lines)?;
                out.write_all(DOCUMENT_CLOSE)
            } else {
                if summary.shown.0 < summary.total.0 {
                    out.write_all(NEWLINE)?;
                    out.write_all(grouped(summary.total).as_bytes())?;
                    out.write_all(ROWS_WORD)?;
                    out.write_all(grouped(summary.shown).as_bytes())?;
                    out.write_all(MORE)?;
                }
                for line in &lines {
                    out.write_all(UNREAD_LINE)?;
                    out.write_all(line.as_bytes())?;
                }
                out.write_all(NEWLINE)
            }
        });
        // The stream is let go before the health is taken: a guard kept through the failure waits on itself.
        let flushed = self
            .out
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .flush();
        if let Err(error) = flushed {
            let mut health = self.health.lock().unwrap_or_else(PoisonError::into_inner);
            if matches!(*health, StreamHealth::Well) {
                *health = refused(&error);
            }
        }
        match &*self.health.lock().unwrap_or_else(PoisonError::into_inner) {
            StreamHealth::Well => Ok(()),
            StreamHealth::Closed => Err(RowSinkError::Closed),
            StreamHealth::Failed(reason) => Err(RowSinkError::Failed {
                reason: reason.clone(),
            }),
        }
    }
}
