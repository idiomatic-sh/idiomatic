//! What the schemas of this output accept and refuse, one row of `CASES` for each input: how a typed cell is printed, and how the rows
//! of an answer are written to a stream that takes them, refuses them or refuses their flush.

use std::io::Write;
use std::sync::{Arc, Mutex};

use ryo_core::outbound::rows::errors::RowsError;
use ryo_core::outbound::rows::interface::RowsPort as _;
use ryo_core::outbound::rows::types::{LentCell, Summary};
use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::relations::constants::NO_SPAN;
use ryo_language::domain::relations::types::{Integer, Offset, Span};
use ryo_values::domain::plan_parts::types::Heading;
use ryo_values::domain::records::types::{Cell, CellText, RowTotal, Unread, Unreadable};
use ryo_values::domain::stored_facts::types::ReadFailure;

use super::super::Format;
use super::{CellSchema, rows};
use crate::cases::{Case, CaseName, Expect, Word, run, selects};

/// A stream the test can read back.
struct Shared(Arc<Mutex<Vec<u8>>>);

impl Write for Shared {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .map_err(|poisoned| std::io::Error::other(poisoned.to_string()))?
            .extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// A stream that refuses everything, as a reader that left does or as a full disk does.
struct Refusing(std::io::ErrorKind);

impl Write for Refusing {
    fn write(&mut self, _bytes: &[u8]) -> std::io::Result<usize> {
        Err(self.0.into())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// A stream that takes every write and refuses the flush, as a buffered stdout does when its reader left or its disk is full.
struct RefusingFlush(std::io::ErrorKind);

impl Write for RefusingFlush {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Err(self.0.into())
    }
}

/// What the stream does with the answer.
#[derive(Clone, Copy, Debug)]
enum Reader {
    /// It takes it.
    Takes,
    /// It refuses every write, with this error.
    Refuses(std::io::ErrorKind),
    /// It takes the writes and refuses the flush, with this error.
    RefusesFlush(std::io::ErrorKind),
}

/// What a row offers the schemas.
#[derive(Clone, Copy, Debug)]
enum Offered {
    /// A cell, printed as text and as JSON.
    Cell(Option<Cell<CellText<'static>>>),
    /// The rows of an answer of two columns and one row, written in a format to a stream that does something with them, with a
    /// summary.
    Rows(Format, Reader, Summary),
}

/// What a row gives the schemas.
type Input = Offered;
/// What was written: the cell's text, then a line break, then its JSON; or the whole answer.
type Value = String;
/// Why the answer could not be written.
type Failure = RowsError;

/// A span value from `start` to `end`.
const fn span(start: u32, end: u32) -> Cell<CellText<'static>> {
    Cell::Span(Span {
        start: Offset(start),
        end: Offset(end),
    })
}

/// A summary of `total` rows of which `shown` were written.
const fn summary(total: u64, shown: u64) -> Summary {
    Summary {
        total: RowTotal(total),
        shown: RowTotal(shown),
    }
}

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_file_is_its_path_as_text_and_a_string_as_json"),
        word: Word::Succeeds,
        input: || Offered::Cell(Some(Cell::File(CellText("src/a.rs")))),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_text_is_itself_and_a_json_string"),
        word: Word::Succeeds,
        input: || Offered::Cell(Some(Cell::Text(CellText("a \"name\"")))),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_number_is_written_in_decimal_and_is_a_json_number"),
        word: Word::Succeeds,
        input: || Offered::Cell(Some(Cell::Number(Integer(-7)))),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_span_is_its_two_ends_and_a_json_object_of_them"),
        word: Word::Succeeds,
        input: || Offered::Cell(Some(span(3, 14))),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_span_the_parser_gave_none_is_unlocated_and_json_null"),
        word: Word::Succeeds,
        input: || Offered::Cell(Some(Cell::Span(NO_SPAN))),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_cell_with_no_value_is_nothing_and_json_null"),
        word: Word::Succeeds,
        input: || Offered::Cell(None),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("text_is_headings_then_a_line_a_row"),
        word: Word::Succeeds,
        input: || Offered::Rows(Format::Terminal, Reader::Takes, summary(1, 1)),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("json_is_one_document_of_typed_cells"),
        word: Word::Succeeds,
        input: || Offered::Rows(Format::Json, Reader::Takes, summary(1, 1)),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_cut_answer_says_how_many_rows_it_has_and_how_many_it_shows"),
        word: Word::Succeeds,
        input: || Offered::Rows(Format::Markdown, Reader::Takes, summary(10_638_912, 1)),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("json_carries_the_count_and_what_was_shown"),
        word: Word::Succeeds,
        input: || Offered::Rows(Format::Json, Reader::Takes, summary(5, 1)),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_reader_that_left_closes_the_answer"),
        word: Word::Refuses,
        input: || {
            Offered::Rows(
                Format::Terminal,
                Reader::Refuses(std::io::ErrorKind::BrokenPipe),
                summary(1, 1),
            )
        },
        expect: Expect::Error(RowsError::Closed),
    },
    Case {
        name: CaseName("a_stream_that_fails_fails_the_answer"),
        word: Word::Fails,
        input: || {
            Offered::Rows(
                Format::Terminal,
                Reader::Refuses(std::io::ErrorKind::StorageFull),
                summary(1, 1),
            )
        },
        expect: Expect::Refusal(|error| selects(matches!(error, RowsError::Failed { .. }))),
    },
    Case {
        name: CaseName("a_reader_that_left_before_the_flush_closes_the_answer"),
        word: Word::Refuses,
        input: || {
            Offered::Rows(
                Format::Json,
                Reader::RefusesFlush(std::io::ErrorKind::BrokenPipe),
                summary(1, 1),
            )
        },
        expect: Expect::Error(RowsError::Closed),
    },
    Case {
        name: CaseName("a_flush_that_fails_fails_the_answer"),
        word: Word::Fails,
        input: || {
            Offered::Rows(
                Format::Json,
                Reader::RefusesFlush(std::io::ErrorKind::StorageFull),
                summary(1, 1),
            )
        },
        expect: Expect::Refusal(|error| selects(matches!(error, RowsError::Failed { .. }))),
    },
];

/// What each succeeding row must write, by its input: the text then the JSON of a cell, or the whole answer.
fn wanted(input: Offered) -> &'static str {
    match input {
        Offered::Cell(Some(Cell::File(_))) => "src/a.rs\n\"src/a.rs\"",
        Offered::Cell(Some(Cell::Text(_))) => "a \"name\"\n\"a \\\"name\\\"\"",
        Offered::Cell(Some(Cell::Number(_))) => "-7\n-7",
        Offered::Cell(Some(Cell::Span(each))) if each == NO_SPAN => "unlocated\nnull",
        Offered::Cell(Some(Cell::Span(_))) => "3..14\n{\"start\":3,\"end\":14}",
        Offered::Cell(None) => "\nnull",
        Offered::Rows(Format::Json, _, each) if each.total.0 == 5 => {
            "{\"columns\":[\"a\",\"b\"],\"rows\":[[\"x\",{\"start\":0,\"end\":2},7,null]],\"count\":5,\"shown\":1,\"unread\":[\"f.rs: why\"]}\n"
        }
        Offered::Rows(Format::Json, _, _) => {
            "{\"columns\":[\"a\",\"b\"],\"rows\":[[\"x\",{\"start\":0,\"end\":2},7,null]],\"count\":1,\"shown\":1,\"unread\":[\"f.rs: why\"]}\n"
        }
        Offered::Rows(_, _, each) if each.total.0 > each.shown.0 => {
            "a\tb\nx\t0..2\t7\t\n10,638,912 rows; showing 1. Use --limit <n> for more.\nunread f.rs: why\n"
        }
        Offered::Rows(_, _, _) => "a\tb\nx\t0..2\t7\t\nunread f.rs: why\n",
    }
}

/// Prints a cell, or writes the rows of an answer, and answers whether what was written is what `wanted` says, as an empty text, or
/// what was written instead.
fn subject(input: Input) -> Result<Value, Failure> {
    let written = match input {
        Offered::Cell(cell) => {
            let json = serde_json::to_string(&CellSchema(cell)).unwrap_or_default();
            format!("{}\n{json}", CellSchema(cell))
        }
        Offered::Rows(format, reader, summary) => {
            let kept = Arc::new(Mutex::new(Vec::new()));
            let out: Box<dyn Write + Send> = match reader {
                Reader::Refuses(kind) => Box::new(Refusing(kind)),
                Reader::RefusesFlush(kind) => Box::new(RefusingFlush(kind)),
                Reader::Takes => Box::new(Shared(Arc::clone(&kept))),
            };
            let writer = rows(out, format);
            writer.begin(&[Heading("a".into()), Heading("b".into())]);
            writer.row(
                &mut [
                    Some(Cell::Text(CellText("x"))),
                    Some(span(0, 2)),
                    Some(Cell::Number(Integer(7))),
                    None,
                ]
                .into_iter()
                .map(LentCell::from),
            );
            writer.end(
                summary,
                &[Unread {
                    path: FilePath("f.rs".into()),
                    reason: Unreadable::Failed(ReadFailure("why".into())),
                }],
            )?;
            kept.lock()
                .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
                .unwrap_or_default()
        }
    };
    Ok(if written == wanted(input) {
        String::new()
    } else {
        written
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}
