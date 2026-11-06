//! The `symbols` service's cases, on values alone.

use super::service::{
    bytes_of, find_hashed, find_or_add_hashed, hash_of, intern, intern_derived, name_made_across,
    place_of, query_intern_derived, text_of,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::names::service::{derived_of, name_count, name_of};
use crate::domain::symbols::types::ByteSlice;
use crate::domain::symbols::types::{QuerySymbols, SymbolStorage, Symbols, SymbolsEngine};
use ryo_relations::domain::relations::types::{
    Bytes, CROSS_FILE, Count, FileId, Offset, Origin, Span, TextHash, WorkerNumber,
};
use ryo_relations::domain::terms::service::term_symbol;
use ryo_relations::domain::terms::types::{DeclarationKind, NameText};

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

/// Query values may alias a source symbol or a prior derived comment, but cannot
/// move the source boundary or reinterpret the comment's derived offsets.
fn query_interning_keeps_source_and_comment_origins() -> usize {
    let source = ByteSlice(b"main # noqa: W605\n");
    let mut symbols = Symbols::default();
    let source_name = intern(&mut symbols, source, Offset(0), Offset(4));
    let comment = intern_derived(&mut symbols, source, NameText(" noqa: W605"));
    let (alias, comment_alias, captured) = {
        let mut query = QuerySymbols::from(&mut symbols);
        (
            query_intern_derived(&mut query, source, NameText("main")),
            query_intern_derived(&mut query, source, NameText(" noqa: W605")),
            query_intern_derived(&mut query, source, NameText("new")),
        )
    };
    let mut arena = Bytes::default();
    let mut places = Vec::new();
    let placed_source = name_of(
        &symbols,
        source,
        source_name,
        FileId(1),
        WorkerNumber(2),
        &mut arena,
        &mut places,
    );
    let source_origin = placed_source
        .and_then(|name| place_of(name, &places, &symbols))
        .is_some_and(|place| {
            place.origin == Origin::Source && place.start == Offset(0) && place.end == Offset(4)
        });
    usize::from(
        alias == source_name
            && comment_alias == comment
            && captured != comment
            && source_origin
            && text_of(&symbols, source, comment).map(|text| text.0) == Some(" noqa: W605")
            && text_of(&symbols, source, captured).map(|text| text.0) == Some("new")
            && symbols.as_ref().storage
                == [
                    SymbolStorage::Source,
                    SymbolStorage::Derived,
                    SymbolStorage::Derived,
                ]
            && name_count(&symbols) == Count(3),
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
    let mut symbols = SymbolsEngine::default();
    let source = ByteSlice(b"leftright");
    let hash = TextHash(7);
    let left = find_or_add_hashed(
        &mut symbols,
        source,
        ByteSlice(b"left"),
        hash,
        SymbolStorage::Source,
        |_| Span {
            start: Offset(0),
            end: Offset(4),
        },
    );
    let right = find_or_add_hashed(
        &mut symbols,
        source,
        ByteSlice(b"right"),
        hash,
        SymbolStorage::Source,
        |_| Span {
            start: Offset(4),
            end: Offset(9),
        },
    );
    let left_again = find_or_add_hashed(
        &mut symbols,
        source,
        ByteSlice(b"left"),
        hash,
        SymbolStorage::Source,
        |_| Span {
            start: Offset(0),
            end: Offset(4),
        },
    );
    let unchanged = left != right
        && left == left_again
        && symbols.start.len() == 2
        && bytes_of(&symbols, source, left) == Some(ByteSlice(b"left"))
        && bytes_of(&symbols, source, right) == Some(ByteSlice(b"right"));
    let symbols = Symbols::from(symbols);
    usize::from(
        unchanged
            && find_hashed(&symbols, source, ByteSlice(b"left"), hash) == Some(left)
            && find_hashed(&symbols, source, ByteSlice(b"right"), hash) == Some(right)
            && find_hashed(&symbols, source, ByteSlice(b"other"), hash).is_none()
            && name_count(&symbols) == Count(2),
    )
}

/// Lookup reads both source and derived names without interning or changing their identities.
fn lookup_of_source_and_derived_names_does_not_add_a_name() -> usize {
    let mut symbols = Symbols::default();
    let source_name = intern(&mut symbols, SOURCE, Offset(0), Offset(4));
    let derived_name = intern_derived(&mut symbols, SOURCE, NameText("src/lib"));
    let before_names = name_count(&symbols);
    let before_derived_bytes = derived_of(&symbols).0.len();
    let found_source = find_hashed(
        &symbols,
        SOURCE,
        ByteSlice(b"main"),
        hash_of(ByteSlice(b"main")),
    );
    let found_derived = find_hashed(
        &symbols,
        SOURCE,
        ByteSlice(b"src/lib"),
        hash_of(ByteSlice(b"src/lib")),
    );
    let missing = find_hashed(
        &symbols,
        SOURCE,
        ByteSlice(b"missing"),
        hash_of(ByteSlice(b"missing")),
    );
    usize::from(
        found_source == Some(source_name)
            && found_derived == Some(derived_name)
            && missing.is_none()
            && name_count(&symbols) == before_names
            && derived_of(&symbols).0.len() == before_derived_bytes,
    )
}

/// File names stay distinct from closed terms with the same text; the term alias is not in the name index.
fn lookup_of_term_text_returns_only_the_file_symbol() -> usize {
    let mut symbols = Symbols::default();
    let dynamic = intern_derived(&mut symbols, ByteSlice(b""), NameText("class"));
    let closed = term_symbol(DeclarationKind::Class);
    let before_names = name_count(&symbols);
    let found = find_hashed(
        &symbols,
        ByteSlice(b""),
        ByteSlice(b"class"),
        hash_of(ByteSlice(b"class")),
    );
    usize::from(
        dynamic != closed
            && found == Some(dynamic)
            && text_of(&symbols, ByteSlice(b""), closed).map(|text| text.0) == Some("class")
            && name_count(&symbols) == before_names,
    )
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("query_interning_keeps_source_and_comment_origins"),
        word: Word::Succeeds,
        input: || query_interning_keeps_source_and_comment_origins,
        expect: Expect::Value(1),
    },
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
    Case {
        name: CaseName("lookup_of_source_and_derived_names_does_not_add_a_name"),
        word: Word::Succeeds,
        input: || lookup_of_source_and_derived_names_does_not_add_a_name,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("lookup_of_term_text_returns_only_the_file_symbol"),
        word: Word::Succeeds,
        input: || lookup_of_term_text_returns_only_the_file_symbol,
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

#[cfg(test)]
mod storage_test;
