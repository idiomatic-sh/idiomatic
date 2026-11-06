//! The `finish` service's cases, on values alone.

use ryo_language::domain::symbols::service::{
    hash_of, intern_derived, name_made_across, places_mut,
};
use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
use ryo_relations::domain::relations::types::{
    CellKind, FileId, Name, NamePlace, Offset, Origin, PlaceNumber, Span, Value as Cell,
};
use ryo_relations::domain::terms::service::unknown_symbol;
use ryo_relations::domain::terms::types::{NameText, ReasonName, UnknownName, UnknownReason};
use ryo_values::domain::files::types::SourceView;

use super::errors::FinishError;
use super::service::{founds, place_all, shown_cell};
use super::types::{Arenas, ColumnNumber, FileLine, Loaded, Point, Resolving};
use crate::cases::{Case, CaseName, Expect, Word, run, selects};

use ryo_values::domain::plan_parts::types::{ColumnIndex, StepId};
use ryo_values::domain::plans::types::Emit;
use ryo_values::domain::row_texts::types::RenderedText;

/// What a row of the cases does with the rows.
#[derive(Clone, Copy)]
enum Scenario {
    /// Makes findings of the rows, with this column as the subject.
    Findings(u32),
    /// Places a finding in a file of two lines.
    Placed,
    /// Reads a name that left its file back from the file.
    Shown,
    /// Shows a value a language could not resolve.
    ShownUnknown,
    /// Shows the retained text of a name carried from project facts.
    CarriedShown,
    /// A complete observation with no table is a refusal, not an empty table.
    MissingPublicationTable,
}

/// What a row of the cases does.
type Input = Scenario;
/// How many findings, or whether the check held.
type Value = usize;
type Failure = FinishError;

const SOURCE: &[u8] = b"fn a() {}\nfn bee() {}\n";

fn rows() -> Table {
    Table {
        kinds: vec![CellKind::File, CellKind::Span],
        count: RowCount(2),
        cells: vec![
            Cell::File(FileId(0)),
            Cell::Span(Span {
                start: Offset(0),
                end: Offset(4),
            }),
            Cell::File(FileId(1)),
            Cell::Span(Span {
                start: Offset(13),
                end: Offset(16),
            }),
        ],
    }
}

fn emit(subject: u32) -> Emit {
    Emit {
        step: StepId(0),
        subject: ColumnIndex(subject),
        key: vec![ColumnIndex(1)],
        headings: Vec::new(),
    }
}

/// Reporting consumes a truthful persisted source-name record; normalization is tested in its owning engine.
fn shown_source_name() -> Value {
    let mut cross = Symbols::default();
    places_mut(&mut cross).push(NamePlace {
        origin: Origin::Source,
        file: FileId(0),
        start: Offset(13),
        end: Offset(16),
    });
    let escaped = Cell::Name(Name {
        hash: hash_of(ByteSlice(b"bee")),
        place: PlaceNumber(0),
    });
    let (paths, arenas) = (Vec::new(), Vec::new());
    let resolving = Resolving {
        unknown: &[],
        paths: &paths,
        arenas: Arenas(&arenas),
        cross: &cross,
    };
    let loaded = Loaded {
        bytes: ryo_values::domain::files::types::FileBytes(SOURCE.to_vec()),
        ..Loaded::default()
    };
    usize::from(shown_text(escaped, &resolving, &loaded).as_deref() == Some("bee"))
}

/// Project names already carried by the engine are read from the retained cross-symbol backing.
fn shown_carried_name() -> Value {
    let mut cross = Symbols::default();
    let symbol = intern_derived(&mut cross, ByteSlice(&[]), NameText("bee"));
    let carried =
        Cell::Name(name_made_across(&cross, symbol).expect("a stored cross-name fixture"));
    let (paths, arenas) = (Vec::new(), Vec::new());
    let resolving = Resolving {
        unknown: &[],
        paths: &paths,
        arenas: Arenas(&arenas),
        cross: &cross,
    };
    usize::from(shown_text(carried, &resolving, &Loaded::default()).as_deref() == Some("bee"))
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
        ryo_values::domain::row_texts::types::RowsTextView(&text),
        shown,
    ) {
        ryo_values::domain::row_texts::types::Cell::Text(piece) => Some(piece.0.to_owned()),
        ryo_values::domain::row_texts::types::Cell::File(_)
        | ryo_values::domain::row_texts::types::Cell::Number(_)
        | ryo_values::domain::row_texts::types::Cell::Span(_) => None,
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
        Scenario::CarriedShown => Ok(shown_carried_name()),
        Scenario::ShownUnknown => Ok(shown_unknown()),
        Scenario::Shown => Ok(shown_source_name()),
        Scenario::MissingPublicationTable => Err(super::service::missing_publication_table()),
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
        name: CaseName("a_carried_project_name_keeps_its_text_when_reported"),
        word: Word::Succeeds,
        input: || Scenario::CarriedShown,
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
    Case {
        name: CaseName("a_complete_observation_without_rows_is_refused"),
        word: Word::Refuses,
        input: || Scenario::MissingPublicationTable,
        expect: Expect::Refusal(|error| selects(matches!(error, FinishError::Publication { .. }))),
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

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

use ryo_values::domain::rows::types::{RowCount, Table};
use ryo_values::domain::runs::types::RuleNumber;
