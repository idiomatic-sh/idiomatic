//! The `names` service's cases, on values alone.

use super::service::{
    begin_name, derived_of, end_name, name_count, name_of, push_name, push_span, push_text,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::symbols::service::{intern, intern_derived, text_of};
use crate::domain::symbols::types::{ByteSlice, Symbols};
use ryo_relations::domain::relations::types::{Bytes, Count, FileId, Offset, WorkerNumber};
use ryo_relations::domain::terms::types::NameText;

/// A scenario: what a row asks of the service.
type Input = fn() -> usize;
/// What a scenario answers: a count.
type Value = usize;
type Failure = ();

/// A name made of pieces is the symbol of its whole text, and the same text twice is one symbol.
fn a_name_of_pieces_is_the_symbol_of_its_whole_text() -> usize {
    let source = ByteSlice(b"ab.cd");
    let mut symbols = Symbols::default();
    let first = intern(&mut symbols, source, Offset(0), Offset(2));
    let start = begin_name(&symbols);
    push_name(&mut symbols, source, first);
    push_text(&mut symbols, NameText("."));
    push_span(&mut symbols, source, Offset(3), Offset(5));
    let made = end_name(&mut symbols, source, start);
    let start = begin_name(&symbols);
    push_text(&mut symbols, NameText("ab.cd"));
    let again = end_name(&mut symbols, source, start);
    usize::from(
        text_of(&symbols, source, made) == Some(NameText("ab.cd"))
            && made == again
            && name_count(&symbols) == Count(2),
    )
}

/// A made name leaves its file as a copy in the worker's text, and a name of the file as a place in the file.
fn a_name_leaves_its_file_as_a_place_or_a_copy() -> usize {
    let source = ByteSlice(b"main");
    let mut symbols = Symbols::default();
    let in_file = intern(&mut symbols, source, Offset(0), Offset(4));
    let made = intern_derived(&mut symbols, source, NameText("made"));
    let (mut arena, mut places) = (Bytes::default(), Vec::new());
    let placed = name_of(
        &symbols,
        source,
        in_file,
        FileId(0),
        WorkerNumber(0),
        &mut arena,
        &mut places,
    );
    let copied = name_of(
        &symbols,
        source,
        made,
        FileId(0),
        WorkerNumber(0),
        &mut arena,
        &mut places,
    );
    usize::from(
        placed.is_some()
            && copied.is_some()
            && places.len() == 2
            && arena.0 == b"made"
            && derived_of(&symbols).0 == b"made",
    )
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_name_of_pieces_is_the_symbol_of_its_whole_text"),
        word: Word::Succeeds,
        input: || a_name_of_pieces_is_the_symbol_of_its_whole_text,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_name_leaves_its_file_as_a_place_or_a_copy"),
        word: Word::Succeeds,
        input: || a_name_leaves_its_file_as_a_place_or_a_copy,
        expect: Expect::Value(1),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    Ok(scenario())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
