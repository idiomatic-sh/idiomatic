//! The `value_cells` service's cases, on values alone.

use crate::domain::table_texts::types::SourceView;
use ryo_language::domain::elements::types::Elements;
use ryo_relations::domain::relations::service::node_id;
use ryo_relations::domain::relations::types::Flag;
use ryo_relations::domain::relations::types::Integer;
use ryo_relations::domain::relations::types::{
    FileId, Name, NamePlace, Offset, Origin, PlaceNumber, Position, Span, TextHash,
    Value as EngineValue,
};
use ryo_language::domain::symbols::service::{intern, places_mut};
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::terms::service::unknown_symbol;
use ryo_relations::domain::terms::types::{ReasonName, UnknownName, UnknownReason};

use super::service::value_cell;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::value_cells::types::Scene;
use ryo_language::domain::files::types::FilePath;
use ryo_values::domain::records::service::lent;
use ryo_values::domain::records::types::RenderedText;
use ryo_values::domain::records::types::{Cell, CellText, RowsTextView};

/// What a row of the cases makes a cell of.
#[derive(Clone, Copy)]
enum Probe {
    /// A value that needs no names, and the cell it is.
    Plain(EngineValue, Option<Cell<CellText<'static>>>),
    /// A name of the file being read, by its symbol.
    Symbol,
    /// A name that left its file, whose text is in this file, and the cell it is in a scene of file 0.
    Name(FileId, Option<Cell<CellText<'static>>>),
    /// An unknown value of reason 5, and whether a language declares the reason, as `dot-import`.
    Unknown(Declared),
}

/// Whether a language declares the reason.
#[derive(Clone, Copy)]
enum Declared {
    /// One does.
    Yes,
    /// None does.
    No,
}

/// The reason a language declares for a value it cannot resolve.
const DECLARED: &[UnknownName] = &[UnknownName {
    reason: UnknownReason(5),
    text: ReasonName("dot-import"),
}];

/// What a row of the cases makes a cell of.
type Input = Probe;
/// Whether the cell made is the cell the row names.
type Value = usize;
type Failure = ();

const SOURCE: &[u8] = b"fn a() {}\nfn bee() {}\n";

fn subject(probe: Input) -> Value {
    let paths = vec![FilePath("lib.rs".into())];
    let mut symbols = Symbols::default();
    let symbol = intern(&mut symbols, ByteSlice(SOURCE), Offset(13), Offset(16));
    let (value, expected, home) = match probe {
        Probe::Plain(value, expected) => (value, expected, FileId(0)),
        Probe::Unknown(declared) => (
            EngineValue::Symbol(unknown_symbol(UnknownReason(5))),
            Some(Cell::Text(CellText(match declared {
                Declared::Yes => "unknown: dot-import",
                Declared::No => "unknown",
            }))),
            FileId(0),
        ),
        Probe::Symbol => (
            EngineValue::Symbol(symbol),
            Some(Cell::Text(CellText("bee"))),
            FileId(0),
        ),
        Probe::Name(home, expected) => (
            EngineValue::Name(Name {
                hash: TextHash(1),
                place: PlaceNumber(0),
            }),
            expected,
            home,
        ),
    };
    places_mut(&mut symbols).push(NamePlace {
        origin: Origin::Source,
        file: home,
        start: Offset(13),
        end: Offset(16),
    });
    let elements = Elements::default();
    let scene = Scene {
        unknown: match probe {
            Probe::Unknown(Declared::Yes) => DECLARED,
            Probe::Plain(..) | Probe::Symbol | Probe::Name(..) | Probe::Unknown(Declared::No) => {
                &[]
            }
        },
        file: FileId(0),
        symbols: &symbols,
        elements: &elements,
        source: SourceView(SOURCE),
        paths: &paths,
    };
    let mut text = RenderedText::default();
    let made = value_cell(&mut text, value, scene);
    usize::from(made.map(|cell| lent(RowsTextView(&text), cell)) == expected)
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_number_is_a_number_cell"),
        word: Word::Succeeds,
        input: || {
            Probe::Plain(
                EngineValue::Number(Integer(-7)),
                Some(Cell::Number(Integer(-7))),
            )
        },
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_flag_is_the_text_true_or_false"),
        word: Word::Succeeds,
        input: || {
            Probe::Plain(
                EngineValue::Flag(Flag::Raised),
                Some(Cell::Text(CellText("true"))),
            )
        },
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_span_is_a_span_cell"),
        word: Word::Succeeds,
        input: || {
            let span = Span {
                start: Offset(3),
                end: Offset(4),
            };
            Probe::Plain(EngineValue::Span(span), Some(Cell::Span(span)))
        },
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_file_is_a_file_cell_of_its_path"),
        word: Word::Succeeds,
        input: || {
            Probe::Plain(
                EngineValue::File(FileId(0)),
                Some(Cell::File(CellText("lib.rs"))),
            )
        },
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("an_unknown_value_is_shown_with_the_name_of_its_reason"),
        word: Word::Succeeds,
        input: || Probe::Unknown(Declared::Yes),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("an_unknown_value_whose_reason_no_language_declares_is_shown_as_unknown"),
        word: Word::Succeeds,
        input: || Probe::Unknown(Declared::No),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_node_is_the_text_of_its_file_and_position"),
        word: Word::Succeeds,
        input: || {
            Probe::Plain(
                EngineValue::Node(node_id(FileId(0), Position(5))),
                Some(Cell::Text(CellText("lib.rs#5"))),
            )
        },
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_file_the_run_does_not_list_is_no_cell"),
        word: Word::Succeeds,
        input: || Probe::Plain(EngineValue::File(FileId(7)), None),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_symbol_is_a_text_cell_of_its_text"),
        word: Word::Succeeds,
        input: || Probe::Symbol,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_name_of_the_file_is_a_text_cell_of_its_bytes"),
        word: Word::Succeeds,
        input: || Probe::Name(FileId(0), Some(Cell::Text(CellText("bee")))),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_name_read_from_another_file_is_no_cell"),
        word: Word::Succeeds,
        input: || Probe::Name(FileId(1), None),
        expect: Expect::Value(1),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| {
        Ok::<_, Failure>(subject(input))
    });
}
