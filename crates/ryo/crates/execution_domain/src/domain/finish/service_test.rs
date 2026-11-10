//! The original producer-name normalization assertions, beside the actual execution-owned implementation.
use super::service::{carry, escape, escape_frame};
use super::types::{Arena, Leaves, Leaving, Retained};
use ryo_language::domain::names::service::{derived_of, places_of};
use ryo_language::domain::symbols::{
    service::{intern, intern_derived, place_of},
    types::{ByteSlice, Symbols},
};
use ryo_relations::domain::{
    relations::types::{CellKind, FileId, Offset, Origin, Value as Cell, WorkerNumber},
    terms::types::NameText,
};
use ryo_values::domain::files::types::SourceView;
use ryo_values::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::rows::types::{RowIndex, Table},
};
use {ryo_values::domain::rows::service::push_row, ryo_values::domain::rows::service::row_in};
#[derive(Clone, Copy)]
enum Scenario {
    Escaped,
    EscapedWithText,
    Carried,
    FramedWithoutText,
}
type Input = Scenario;
type Value = usize;
type Failure = std::convert::Infallible;
const SOURCE: &[u8] = b"fn a() {}\nfn bee() {}\n";
const CASES: &[Case<Input, Value, Failure>] = &[
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
        name: CaseName("a_node_without_text_leaves_in_a_frame_and_a_symbol_of_no_name_does_not"),
        word: Word::Succeeds,
        input: || Scenario::FramedWithoutText,
        expect: Expect::Value(1),
    },
];
fn names(name: Cell) -> Table {
    let mut table = Table {
        kinds: vec![CellKind::Symbol],
        ..Table::default()
    };
    push_row(&mut table, &[name]);
    table
}
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
    let text = carried.and_then(|cell| {
        let Cell::Name(name) = cell else {
            return None;
        };
        let place = place_of(name, places_of(&cross), &cross)?;
        assert!(matches!(place.origin, Origin::Derived(_)));
        derived_of(&cross)
            .0
            .get(usize::try_from(place.start.0).ok()?..usize::try_from(place.end.0).ok()?)
    });
    usize::from(carried.is_some() && carried == escaped && text == Some(b"bee".as_slice()))
}
/// A frame of rows whose `text` is `NO_SYMBOL` (a node, which has no text) is published with it as it is; a symbol the producer
/// cannot name is still refused.
fn framed_without_text() -> Value {
    use ryo_relations::domain::relations::constants::NO_SYMBOL;
    use ryo_relations::domain::terms::types::Symbol;
    let symbols = Symbols::default();
    let frame = |cell: Cell| {
        let mut table = names(cell);
        let published = escape_frame(
            &mut table,
            Leaving {
                symbols: &symbols,
                source: SourceView(SOURCE),
                file: FileId(0),
                worker: WorkerNumber(2),
                text: Leaves::WithText,
            },
            Arena(&mut Vec::new()),
        );
        (published, table)
    };
    let (published, table) = frame(Cell::Symbol(NO_SYMBOL));
    let kept = published.is_ok_and(|places| places.is_empty())
        && matches!(row_in(&table, RowIndex(0)), Some([Cell::Symbol(symbol)]) if *symbol == NO_SYMBOL);
    let (unnamed, _) = frame(Cell::Symbol(Symbol(7)));
    usize::from(kept && unnamed == Err(super::types::PublicationReason::ProducerSymbolUnescaped))
}
fn subject(scenario: Input) -> Result<Value, Failure> {
    Ok(match scenario {
        Scenario::Escaped => escaped(),
        Scenario::EscapedWithText => escaped_with_text(),
        Scenario::Carried => carried(),
        Scenario::FramedWithoutText => framed_without_text(),
    })
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
