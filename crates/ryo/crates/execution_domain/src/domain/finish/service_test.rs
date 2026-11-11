//! The original producer-name normalization assertions, beside the actual execution-owned implementation.
use std::borrow::Cow;

use super::service::{carry, escape, escape_frame};
use super::types::{Arena, Leaves, Leaving, PublicationReason, Retained};
use ryo_language::domain::names::service::{derived_of, places_of};
use ryo_language::domain::symbols::{
    service::{intern, intern_derived, place_of},
    types::{ByteSlice, Symbols},
};
use ryo_relations::domain::relations::constants::NO_SYMBOL;
use ryo_relations::domain::terms::types::Symbol;
use ryo_relations::domain::{
    relations::types::{
        CROSS_FILE, CellKind, FileId, NamePlace, Offset, Origin, Value as Cell, WorkerNumber,
    },
    terms::types::NameText,
};
use ryo_values::domain::files::types::SourceView;
use ryo_values::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::rows::types::{RowIndex, Table},
};
use {ryo_values::domain::rows::service::push_row, ryo_values::domain::rows::service::row_in};
/// What is published of the name `bee` of file 0 (bytes 13 to 16 of `SOURCE`).
#[derive(Clone, Copy, Debug)]
enum Handover {
    /// The name leaves its file by `escape`, from worker 2 with its text or worker 0 without.
    Escape(Leaves),
    /// The project writes the name, `carry` hands it across, and a file escapes the same name.
    Carry,
    /// A frame of one row whose symbol is this, published by worker 2 with its text.
    Frame(Symbol),
}

/// What leaving its file made of a name.
#[derive(Debug, PartialEq)]
enum Left {
    /// The place number of the name cell the row now holds (none when it is not a name), the places published, and the
    /// text the worker kept.
    Escaped {
        place: Option<u32>,
        places: Cow<'static, [NamePlace]>,
        arena: Cow<'static, [u8]>,
    },
    /// Whether the carried cell is the escaped one, where the carried name's text is, and the text.
    Carried {
        same: bool,
        origin: Option<Origin>,
        text: Option<Cow<'static, [u8]>>,
    },
    /// How many places the frame published, and whether its row still holds the symbol.
    Framed { places: usize, kept: bool },
}

type Input = Handover;
type Value = Left;
type Failure = PublicationReason;
const SOURCE: &[u8] = b"fn a() {}\nfn bee() {}\n";
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_name_leaves_its_file_with_its_text_when_the_steps_read_it"),
        word: Word::Succeeds,
        input: || Handover::Escape(Leaves::WithText),
        expect: Expect::Value(Left::Escaped {
            place: Some(0),
            places: Cow::Borrowed(&[NamePlace {
                origin: Origin::Derived(WorkerNumber(2)),
                file: FileId(0),
                start: Offset(0),
                end: Offset(3),
            }]),
            arena: Cow::Borrowed(b"bee"),
        }),
    },
    Case {
        name: CaseName("a_name_leaves_its_file_as_a_place"),
        word: Word::Succeeds,
        input: || Handover::Escape(Leaves::WithoutText),
        expect: Expect::Value(Left::Escaped {
            place: Some(0),
            places: Cow::Borrowed(&[NamePlace {
                origin: Origin::Source,
                file: FileId(0),
                start: Offset(13),
                end: Offset(16),
            }]),
            arena: Cow::Borrowed(b""),
        }),
    },
    Case {
        name: CaseName("a_name_the_project_wrote_compares_with_the_name_a_file_handed_over"),
        word: Word::Succeeds,
        input: || Handover::Carry,
        expect: Expect::Value(Left::Carried {
            same: true,
            origin: Some(Origin::Derived(CROSS_FILE)),
            text: Some(Cow::Borrowed(b"bee")),
        }),
    },
    Case {
        name: CaseName("a_node_without_text_leaves_in_a_frame_and_a_symbol_of_no_name_does_not"),
        word: Word::Succeeds,
        input: || Handover::Frame(NO_SYMBOL),
        expect: Expect::Value(Left::Framed {
            places: 0,
            kept: true,
        }),
    },
    Case {
        name: CaseName("a_symbol_of_no_name_does_not_leave_in_a_frame"),
        word: Word::Refuses,
        input: || Handover::Frame(Symbol(7)),
        expect: Expect::Error(PublicationReason::ProducerSymbolUnescaped),
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
fn escaped(text: Leaves) -> Value {
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
            worker: WorkerNumber(match text {
                Leaves::WithText => 2,
                Leaves::WithoutText => 0,
            }),
            text,
        },
        Retained {
            arena: Arena(&mut arena),
            places: &mut places,
        },
    );
    let place = match row_in(&table, RowIndex(0)) {
        Some([Cell::Name(kept)]) => Some(kept.place.0),
        _ => None,
    };
    Left::Escaped {
        place,
        places: places.into(),
        arena: arena.into(),
    }
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
    let place = carried.and_then(|cell| {
        let Cell::Name(name) = cell else {
            return None;
        };
        place_of(name, places_of(&cross), &cross)
    });
    let text = place.and_then(|place| {
        derived_of(&cross)
            .0
            .get(usize::try_from(place.start.0).ok()?..usize::try_from(place.end.0).ok()?)
            .map(|text| Cow::Owned(text.to_vec()))
    });
    Left::Carried {
        same: carried.is_some() && carried == escaped,
        origin: place.map(|place| place.origin),
        text,
    }
}
/// A frame of rows whose `text` is `NO_SYMBOL` (a node, which has no text) is published with it as it is; a symbol the producer
/// cannot name is refused.
fn framed(symbol: Symbol) -> Result<Value, Failure> {
    let symbols = Symbols::default();
    let mut table = names(Cell::Symbol(symbol));
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
    )?;
    Ok(Left::Framed {
        places: published.len(),
        kept: matches!(row_in(&table, RowIndex(0)), Some([Cell::Symbol(kept)]) if *kept == symbol),
    })
}
fn subject(handover: Input) -> Result<Value, Failure> {
    match handover {
        Handover::Escape(text) => Ok(escaped(text)),
        Handover::Carry => Ok(carried()),
        Handover::Frame(symbol) => framed(symbol),
    }
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
