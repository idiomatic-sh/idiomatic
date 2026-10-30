//! The `symbols` service's cases, on values alone.

use super::service::{
    bytes_of, derived_of, find_or_add_hashed, intern, intern_derived, name_count, name_made_across,
    name_of, place_of, text_of,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::types::{
    Bytes, CROSS_FILE, Count, FileId, Offset, Origin, Span, TextHash, WorkerNumber,
};
use crate::domain::symbols::types::ByteSlice;
use crate::domain::symbols::types::{Symbols, SymbolsEngine};
use crate::domain::terms::types::NameText;

/// A scenario: what a row asks of the service.
type Input = fn() -> usize;
/// What a scenario answers: a count.
type Value = usize;
type Failure = ();

const SOURCE: ByteSlice<'static> = ByteSlice(b"mainmain");

fn a_name_made_after_the_files_were_read_keeps_its_text_where_it_was_made() -> usize {
    let mut symbols = Symbols::default();
    let made = intern_derived(&mut symbols, ByteSlice(&[]), NameText("crates/x"));
    let again = intern_derived(&mut symbols, ByteSlice(&[]), NameText("crates/x"));
    let name = name_made_across(&symbols, made);
    let place = name.and_then(|each| place_of(each, &[], &symbols));
    let text = place.and_then(|each| {
        derived_of(&symbols).0.get(
            usize::try_from(each.start.0).unwrap_or(0)..usize::try_from(each.end.0).unwrap_or(0),
        )
    });
    usize::from(
        made == again
            && place.is_some_and(|each| each.origin == Origin::Derived(CROSS_FILE))
            && text == Some(b"crates/x".as_slice())
            && name == name_made_across(&symbols, again),
    )
}

fn the_same_text_twice_is_one_symbol_and_nothing_is_copied() -> usize {
    let mut symbols = Symbols::default();
    let first = intern(&mut symbols, SOURCE, Offset(0), Offset(4));
    let again = intern(&mut symbols, SOURCE, Offset(4), Offset(8));
    usize::from(
        first == again
            && name_count(&symbols) == Count(1)
            && derived_of(&symbols).0.is_empty()
            && text_of(&symbols, SOURCE, first).map(|text| text.0) == Some("main"),
    )
}

fn a_derived_name_is_kept_beside_the_names_of_the_file() -> usize {
    let mut symbols = Symbols::default();
    let read = intern(&mut symbols, SOURCE, Offset(0), Offset(4));
    let made = intern_derived(&mut symbols, SOURCE, NameText("src/lib"));
    let again = intern_derived(&mut symbols, SOURCE, NameText("src/lib"));
    let same = intern_derived(&mut symbols, SOURCE, NameText("main"));
    usize::from(
        read != made
            && made == again
            && same == read
            && text_of(&symbols, SOURCE, made).map(|text| text.0) == Some("src/lib"),
    )
}

fn a_name_leaves_its_file_as_a_place_and_a_made_name_as_a_copy() -> usize {
    let mut symbols = Symbols::default();
    let read = intern(&mut symbols, SOURCE, Offset(0), Offset(4));
    let made = intern_derived(&mut symbols, SOURCE, NameText("src/lib"));
    let mut arena = Bytes::default();
    let mut places = Vec::new();
    let out = name_of(
        &symbols,
        SOURCE,
        read,
        FileId(2),
        WorkerNumber(1),
        &mut arena,
        &mut places,
    );
    let copied = name_of(
        &symbols,
        SOURCE,
        made,
        FileId(2),
        WorkerNumber(1),
        &mut arena,
        &mut places,
    );
    let at_source = out
        .and_then(|name| place_of(name, &places, &symbols))
        .is_some_and(|place| {
            place.origin == Origin::Source
                && place.start == Offset(0)
                && place.end == Offset(4)
                && arena.0.len() == 7
        });
    let kept = copied
        .and_then(|name| place_of(name, &places, &symbols))
        .is_some_and(|place| {
            place.origin == Origin::Derived(WorkerNumber(1)) && arena.0 == b"src/lib"
        });
    usize::from(at_source && kept)
}

/// Two texts that hash alike, in one file, stay two names: a hit on the hash is the name only when its text is the text.
fn two_texts_of_one_hash_in_one_file_are_two_symbols() -> usize {
    let mut symbols = SymbolsEngine {
        sources: Count(2),
        ..SymbolsEngine::default()
    };
    let source = ByteSlice(b"leftright");
    let hash = TextHash(7);
    let left = find_or_add_hashed(&mut symbols, source, ByteSlice(b"left"), hash, |_| Span {
        start: Offset(0),
        end: Offset(4),
    });
    let right = find_or_add_hashed(&mut symbols, source, ByteSlice(b"right"), hash, |_| Span {
        start: Offset(4),
        end: Offset(9),
    });
    let left_again = find_or_add_hashed(&mut symbols, source, ByteSlice(b"left"), hash, |_| Span {
        start: Offset(0),
        end: Offset(4),
    });
    usize::from(
        left != right
            && left == left_again
            && symbols.start.len() == 2
            && bytes_of(&symbols, source, left) == Some(ByteSlice(b"left"))
            && bytes_of(&symbols, source, right) == Some(ByteSlice(b"right")),
    )
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_name_made_after_the_files_were_read_keeps_its_text_where_it_was_made"),
        word: Word::Succeeds,
        input: || a_name_made_after_the_files_were_read_keeps_its_text_where_it_was_made,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("the_same_text_twice_is_one_symbol_and_nothing_is_copied"),
        word: Word::Succeeds,
        input: || the_same_text_twice_is_one_symbol_and_nothing_is_copied,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_derived_name_is_kept_beside_the_names_of_the_file"),
        word: Word::Succeeds,
        input: || a_derived_name_is_kept_beside_the_names_of_the_file,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_name_leaves_its_file_as_a_place_and_a_made_name_as_a_copy"),
        word: Word::Succeeds,
        input: || a_name_leaves_its_file_as_a_place_and_a_made_name_as_a_copy,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("two_texts_of_one_hash_in_one_file_are_two_symbols"),
        word: Word::Succeeds,
        input: || two_texts_of_one_hash_in_one_file_are_two_symbols,
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
