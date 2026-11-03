//! The `finish` service's cases, on values alone.

use crate::domain::table_texts::types::SourceView;
use ryo_relations::domain::relations::types::{
    CellKind, FileId, Offset, Origin, Span, Value as Cell, WorkerNumber,
};
use ryo_language::domain::symbols::service::{intern, intern_derived, places_mut};
use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
use ryo_relations::domain::terms::service::unknown_symbol;
use ryo_relations::domain::terms::types::{NameText, ReasonName, UnknownName, UnknownReason};

use super::errors::FinishError;
use super::service::{carry, escape, founds, place_all, shown_cell};
use super::types::{
    Arena, Arenas, ColumnNumber, FileLine, Leaves, Leaving, Loaded, Point, Resolving, Retained,
    RuleNumber,
};
use crate::cases::{Case, CaseName, Expect, Word, run, selects};
use crate::domain::table_cells::types::RowIndex;
use crate::domain::table_model::errors::TableModelError;
use crate::domain::table_model::service::{push_row, row_in};
use crate::domain::table_model::types::Table;
use ryo_values::domain::plan_parts::types::{ColumnIndex, StepId};
use ryo_values::domain::plans::types::Emit;
use ryo_values::domain::records::types::RenderedText;

/// What a row of the cases does with the rows.
#[derive(Clone, Copy)]
enum Scenario {
    /// Makes findings of the rows, with this column as the subject.
    Findings(u32),
    /// Places a finding in a file of two lines.
    Placed,
    /// Sends a name out of its file.
    Escaped,
    /// Sends a name out of its file with its text, for steps that read the text after the files are read.
    EscapedWithText,
    /// Reads a name that left its file back from the file.
    Shown,
    /// Shows a value a language could not resolve.
    ShownUnknown,
    /// Carries a name the project wrote to the steps after the files, where it compares with the name a file handed over.
    Carried,
}

/// What a row of the cases does.
type Input = Scenario;
/// How many findings, or whether the check held.
type Value = usize;
type Failure = FinishError;

const SOURCE: &[u8] = b"fn a() {}\nfn bee() {}\n";

fn rows() -> Table {
    let mut table = Table {
        kinds: vec![CellKind::File, CellKind::Span],
        ..Table::default()
    };
    push_row(
        &mut table,
        &[
            Cell::File(FileId(0)),
            Cell::Span(Span {
                start: Offset(0),
                end: Offset(4),
            }),
        ],
    );
    push_row(
        &mut table,
        &[
            Cell::File(FileId(1)),
            Cell::Span(Span {
                start: Offset(13),
                end: Offset(16),
            }),
        ],
    );
    table
}

/// A table of one name.
fn names(name: Cell) -> Table {
    let mut table = Table {
        kinds: vec![CellKind::Symbol],
        ..Table::default()
    };
    push_row(&mut table, &[name]);
    table
}

fn emit(subject: u32) -> Emit {
    Emit {
        step: StepId(0),
        subject: ColumnIndex(subject),
        key: vec![ColumnIndex(1)],
        headings: Vec::new(),
    }
}

/// A name the project wrote, carried among the cross-file names, is the cell a name that escaped its file is, and reads as its text.
fn carried() -> Value {
    let (mut project, mut cross) = (Symbols::default(), Symbols::default());
    let written = intern_derived(&mut project, ByteSlice(&[]), NameText("bee"));
    let mut table = names(Cell::Symbol(written));
    carry(&mut table, &project, &mut cross);
    let mut handed = Symbols::default();
    let name = intern(&mut handed, ByteSlice(SOURCE), Offset(13), Offset(16));
    let mut file = names(Cell::Symbol(name));
    escape(
        &mut file,
        Leaving {
            symbols: &handed,
            source: SourceView(SOURCE),
            file: FileId(0),
            worker: WorkerNumber(0),
            text: Leaves::WithoutText,
        },
        Retained {
            arena: Arena(&mut Vec::new()),
            places: &mut Vec::new(),
        },
    );
    let first = |rows: &Table| row_in(rows, RowIndex(0)).and_then(<[Cell]>::first).copied();
    let (carried, escaped) = (first(&table), first(&file));
    let (paths, arenas) = (Vec::new(), Vec::new());
    let resolving = Resolving {
        unknown: &[],
        paths: &paths,
        arenas: Arenas(&arenas),
        cross: &cross,
    };
    let text = carried.and_then(|each| shown_text(each, &resolving, &Loaded::default()));
    usize::from(carried.is_some() && carried == escaped && text.as_deref() == Some("bee"))
}

/// A name that leaves with its text keeps its bytes in the arena, placed as the worker's own.
fn escaped_with_text() -> Value {
    let mut symbols = Symbols::default();
    let name = intern(&mut symbols, ByteSlice(SOURCE), Offset(13), Offset(16));
    let mut table = names(Cell::Symbol(name));
    let mut arena = Vec::new();
    let mut places = Vec::new();
    escape(
        &mut table,
        Leaving {
            symbols: &symbols,
            source: SourceView(SOURCE),
            file: FileId(0),
            worker: WorkerNumber(2),
            text: Leaves::WithText,
        },
        Retained {
            arena: Arena(&mut arena),
            places: &mut places,
        },
    );
    let kept = places.first().is_some_and(|place| {
        place.origin == Origin::Derived(WorkerNumber(2))
            && arena.get(
                usize::try_from(place.start.0).unwrap_or(0)
                    ..usize::try_from(place.end.0).unwrap_or(0),
            ) == Some(b"bee".as_slice())
    });
    usize::from(kept)
}

/// A name that leaves without its text keeps its place in the source and nothing in the arena.
fn escaped() -> Value {
    let mut symbols = Symbols::default();
    let name = intern(&mut symbols, ByteSlice(SOURCE), Offset(13), Offset(16));
    let mut table = names(Cell::Symbol(name));
    let mut arena = Vec::new();
    let mut places = Vec::new();
    escape(
        &mut table,
        Leaving {
            symbols: &symbols,
            source: SourceView(SOURCE),
            file: FileId(0),
            worker: WorkerNumber(0),
            text: Leaves::WithoutText,
        },
        Retained {
            arena: Arena(&mut arena),
            places: &mut places,
        },
    );
    usize::from(
        matches!(row_in(&table, RowIndex(0)), Some([Cell::Name(kept)]) if kept.place.0 == 0)
            && places.len() == 1
            && places[0].start == Offset(13)
            && places[0].end == Offset(16)
            && arena.is_empty(),
    )
}

/// Whether a value a language could not resolve shows as `unknown`, as 1 or 0.
fn shown_unknown() -> usize {
    let cross = Symbols::default();
    let (paths, arenas) = (Vec::new(), Vec::new());
    let resolving = Resolving {
        unknown: &[],
        paths: &paths,
        arenas: Arenas(&arenas),
        cross: &cross,
    };
    let unknown = Cell::Symbol(unknown_symbol(UnknownReason(1)));
    let text = shown_text(unknown, &resolving, &Loaded::default());
    // The same value, once a language declares its reason, is shown with the reason's name.
    let declared = [UnknownName {
        reason: UnknownReason(1),
        text: ReasonName("dot-import"),
    }];
    let named = Resolving {
        unknown: &declared,
        ..resolving
    };
    let with_name = shown_text(unknown, &named, &Loaded::default());
    usize::from(
        text.as_deref() == Some("unknown") && with_name.as_deref() == Some("unknown: dot-import"),
    )
}

/// The text of the text cell `value` is in a report; none for any other cell.
fn shown_text(value: Cell, resolving: &Resolving<'_>, loaded: &Loaded) -> Option<String> {
    let mut text = RenderedText::default();
    let shown = shown_cell(&mut text, value, resolving, loaded)?;
    match ryo_values::domain::records::service::lent(
        ryo_values::domain::records::types::RowsTextView(&text),
        shown,
    ) {
        ryo_values::domain::records::types::Cell::Text(piece) => Some(piece.0.to_owned()),
        ryo_values::domain::records::types::Cell::File(_)
        | ryo_values::domain::records::types::Cell::Number(_)
        | ryo_values::domain::records::types::Cell::Span(_) => None,
    }
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    match scenario {
        Scenario::Findings(subject) => {
            Ok(founds(RuleNumber(0), &rows(), &emit(subject), FileId(0))?.len())
        }
        Scenario::Placed => {
            let mut found = founds(RuleNumber(0), &rows(), &emit(0), FileId(0))?;
            place_all(
                &mut found,
                &emit(0),
                SourceView(SOURCE),
                &Symbols::default(),
            );
            let (first, second) = (found[0].place, found[1].place);
            Ok(usize::from(
                first
                    == Point {
                        line: FileLine(1),
                        column: ColumnNumber(1),
                    }
                    && second
                        == Point {
                            line: FileLine(2),
                            column: ColumnNumber(4),
                        },
            ))
        }
        Scenario::Escaped => Ok(escaped()),
        Scenario::EscapedWithText => Ok(escaped_with_text()),
        Scenario::Carried => Ok(carried()),
        Scenario::ShownUnknown => Ok(shown_unknown()),
        Scenario::Shown => {
            let mut symbols = Symbols::default();
            let name = intern(&mut symbols, ByteSlice(SOURCE), Offset(13), Offset(16));
            let mut table = names(Cell::Symbol(name));
            let mut cross = Symbols::default();
            escape(
                &mut table,
                Leaving {
                    symbols: &symbols,
                    source: SourceView(SOURCE),
                    file: FileId(0),
                    worker: WorkerNumber(0),
                    text: Leaves::WithoutText,
                },
                Retained {
                    arena: Arena(&mut Vec::new()),
                    places: places_mut(&mut cross),
                },
            );
            let (paths, arenas) = (Vec::new(), Vec::new());
            let resolving = Resolving {
                unknown: &[],
                paths: &paths,
                arenas: Arenas(&arenas),
                cross: &cross,
            };
            let escaped = row_in(&table, RowIndex(0))
                .and_then(<[Cell]>::first)
                .copied()
                .ok_or(FinishError::Table(TableModelError::NoSuchColumn {
                    index: 0,
                }))?;
            let loaded = Loaded {
                bytes: ryo_values::domain::files::types::FileBytes(SOURCE.to_vec()),
                ..Loaded::default()
            };
            let text = shown_text(escaped, &resolving, &loaded);
            Ok(usize::from(text.as_deref() == Some("bee")))
        }
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("each_row_is_a_finding"),
        word: Word::Succeeds,
        input: || Scenario::Findings(0),
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("a_finding_is_placed_by_line_and_character"),
        word: Word::Succeeds,
        input: || Scenario::Placed,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_name_leaves_its_file_with_its_text_when_the_steps_read_it"),
        word: Word::Succeeds,
        input: || Scenario::EscapedWithText,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_name_leaves_its_file_as_a_place"),
        word: Word::Succeeds,
        input: || Scenario::Escaped,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_name_the_project_wrote_compares_with_the_name_a_file_handed_over"),
        word: Word::Succeeds,
        input: || Scenario::Carried,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_value_a_language_could_not_resolve_shows_as_unknown"),
        word: Word::Succeeds,
        input: || Scenario::ShownUnknown,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_name_is_read_back_from_its_file"),
        word: Word::Succeeds,
        input: || Scenario::Shown,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_subject_column_the_rows_lack_is_refused"),
        word: Word::Fails,
        input: || Scenario::Findings(7),
        expect: Expect::Refusal(|error| selects(matches!(error, FinishError::Table(..)))),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}
