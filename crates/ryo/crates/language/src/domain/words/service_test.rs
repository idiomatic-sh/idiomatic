//! The `words` service's cases, on values alone.

use super::service::{
    decode_kind, decode_places, decode_value, encode_kind, encode_places, encode_value,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::types::{
    Bits, CellKind, FileId, Flag, Integer, Name, NamePlace, NodeId, Offset, Origin, PlaceNumber,
    Span, TextHash, Value as Cell, Words, WorkerNumber,
};
use crate::domain::terms::types::Symbol;

/// What a row of the cases writes and reads back.
type Input = fn() -> Vec<Cell>;
/// How many of the values came back as they were written.
type Value = usize;
type Failure = ();

fn every_kind_of_value() -> Vec<Cell> {
    vec![
        Cell::Node(NodeId(7)),
        Cell::Symbol(Symbol(3)),
        Cell::Name(Name {
            hash: TextHash(77),
            place: PlaceNumber(2),
        }),
        Cell::Name(Name {
            hash: TextHash(88),
            place: PlaceNumber(0),
        }),
        Cell::File(FileId(4)),
        Cell::Span(Span {
            start: Offset(4),
            end: Offset(9),
        }),
        Cell::Number(Integer(-5)),
        Cell::Flag(Flag::Raised),
    ]
}

fn subject(values: Input) -> Result<Value, Failure> {
    let values = values();
    let mut words = Words::default();
    for value in &values {
        encode_value(*value, &mut words);
    }
    let mut read = words.0.iter().copied().map(Bits);
    let same = values
        .iter()
        .filter(|value| decode_value(&mut read) == Some(**value))
        .count();
    let mut kinds = Words::default();
    for kind in [
        CellKind::Node,
        CellKind::Symbol,
        CellKind::File,
        CellKind::Span,
        CellKind::Number,
        CellKind::Flag,
    ] {
        encode_kind(kind, &mut kinds);
    }
    let mut read_kinds = kinds.0.iter().copied().map(Bits);
    let kinds_same = [
        CellKind::Node,
        CellKind::Symbol,
        CellKind::File,
        CellKind::Span,
        CellKind::Number,
        CellKind::Flag,
    ]
    .into_iter()
    .filter(|kind| decode_kind(&mut read_kinds) == Some(*kind))
    .count();
    Ok(same * 10 + kinds_same)
}

/// A name that left its file is equal to another with the same hash, wherever its text is; the
/// words keep the number of its place all the same.
fn names_keep_their_place() -> Vec<Cell> {
    vec![Cell::Name(Name {
        hash: TextHash(5),
        place: PlaceNumber(9),
    })]
}

/// Places are written and read back as they were: where the text is.
fn places_survive_being_words() -> Vec<Cell> {
    let places = [
        NamePlace {
            origin: Origin::Derived(WorkerNumber(3)),
            file: FileId(1),
            start: Offset(0),
            end: Offset(4),
        },
        NamePlace {
            origin: Origin::Source,
            file: FileId(2),
            start: Offset(5),
            end: Offset(9),
        },
    ];
    let mut words = Words::default();
    encode_places(&places, &mut words);
    let mut read = Vec::new();
    let ok = decode_places(&mut words.0.iter().copied().map(Bits), &mut read).is_some()
        && read == places;
    vec![Cell::Number(Integer(1)); usize::from(ok)]
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_kind_of_value_survives_being_words"),
        word: Word::Succeeds,
        input: || every_kind_of_value,
        expect: Expect::Value(86),
    },
    Case {
        name: CaseName("places_survive_being_words"),
        word: Word::Succeeds,
        input: || places_survive_being_words,
        expect: Expect::Value(16),
    },
    Case {
        name: CaseName("a_name_survives_being_words"),
        word: Word::Succeeds,
        input: || names_keep_their_place,
        expect: Expect::Value(16),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
