//! The `symbols` service's cases for the storage a name keeps across interleaved sources, derived names and pools.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::names::service::{derived_of, name_count, name_of};
use crate::domain::symbols::service::{
    bytes_of, find_hashed, find_or_add_hashed, hash_of, intern, intern_derived, name_made_across,
    place_of, query_intern_derived, text_of,
};
use crate::domain::symbols::types::ByteSlice;
use crate::domain::symbols::types::{
    Generation, QuerySymbols, SymbolStorage, Symbols, SymbolsEngine,
};
use ryo_relations::domain::relations::types::{
    Bytes, CROSS_FILE, Count, FileId, Offset, Origin, Span, TextHash, WorkerNumber,
};
use ryo_relations::domain::terms::service::term_symbol;
use ryo_relations::domain::declaration_kind::types::DeclarationKind;
use ryo_relations::domain::terms::types::NameText;

/// A scenario: what a row asks of the service.
type Input = fn() -> usize;
/// What a scenario answers: a count.
type Value = usize;
type Failure = ();

/// Late source admission must not reinterpret an existing derived symbol's offsets.
fn interleaved_source_and_derived_names_keep_text_and_retention() -> usize {
    let source = ByteSlice(b"x.Z");
    let mut symbols = Symbols::default();
    let derived = intern_derived(&mut symbols, source, NameText("x.A"));
    let original = intern(&mut symbols, source, Offset(0), Offset(3));
    assert_eq!(text_of(&symbols, source, derived), Some(NameText("x.A")));
    assert_eq!(text_of(&symbols, source, original), Some(NameText("x.Z")));
    assert_eq!(
        find_hashed(
            &symbols,
            source,
            ByteSlice(b"x.A"),
            hash_of(ByteSlice(b"x.A"))
        ),
        Some(derived)
    );
    assert_eq!(
        find_hashed(
            &symbols,
            source,
            ByteSlice(b"x.Z"),
            hash_of(ByteSlice(b"x.Z"))
        ),
        Some(original)
    );
    let mut arena = Bytes::default();
    let mut places = Vec::new();
    let retained = name_of(
        &symbols,
        source,
        derived,
        FileId(0),
        WorkerNumber(0),
        &mut arena,
        &mut places,
    )
    .expect("derived name remains retainable");
    let retained_hash = retained.hash;
    assert_eq!(retained_hash, hash_of(ByteSlice(b"x.A")));
    assert_eq!(arena.0, b"x.A");
    assert_eq!(
        place_of(retained, &places, &symbols)
            .expect("retained place")
            .origin,
        Origin::Derived(WorkerNumber(0))
    );
    let retained_source = name_of(
        &symbols,
        source,
        original,
        FileId(0),
        WorkerNumber(0),
        &mut arena,
        &mut places,
    )
    .expect("source remains retainable");
    let retained_source_hash = retained_source.hash;
    assert_eq!(retained_source_hash, hash_of(ByteSlice(b"x.Z")));
    assert_eq!(
        place_of(retained_source, &places, &symbols)
            .expect("source place")
            .origin,
        Origin::Source
    );
    assert_eq!(arena.0, b"x.A", "source retention does not copy bytes");
    1
}

/// Finding a derived name from source bytes must retain that symbol's original storage.
fn a_source_alias_does_not_reclassify_existing_derived_storage() -> usize {
    let source = ByteSlice(b"x.A");
    let mut symbols = Symbols::default();
    let prefix = intern_derived(&mut symbols, source, NameText("prefix"));
    let derived = intern_derived(&mut symbols, source, NameText("x.A"));
    let alias = intern(&mut symbols, source, Offset(0), Offset(3));
    assert_eq!(alias, derived);
    assert_eq!(text_of(&symbols, source, prefix), Some(NameText("prefix")));
    assert_eq!(text_of(&symbols, source, derived), Some(NameText("x.A")));
    assert_eq!(name_count(&symbols), Count(2));
    1
}

/// Composed names and reused pools obey the same storage identity as ordinary lookup.
fn interleaved_names_compose_and_clear_without_changing_storage() -> usize {
    use crate::domain::names::service::{begin_name, end_name, push_name};
    use crate::domain::symbols::service::clear_symbols;
    let source = ByteSlice(b"x.Z");
    let mut symbols = Symbols::default();
    let derived = intern_derived(&mut symbols, source, NameText("x.A"));
    let original = intern(&mut symbols, source, Offset(0), Offset(3));
    let start = begin_name(&symbols);
    push_name(&mut symbols, source, derived);
    push_name(&mut symbols, source, original);
    let composed = end_name(&mut symbols, source, start);
    assert_eq!(
        text_of(&symbols, source, composed),
        Some(NameText("x.Ax.Z"))
    );
    assert_eq!(
        intern_derived(&mut symbols, source, NameText("x.Ax.Z")),
        composed
    );
    clear_symbols(&mut symbols);
    assert_eq!(name_count(&symbols), Count(0));
    let original = intern(&mut symbols, source, Offset(0), Offset(3));
    assert_eq!(text_of(&symbols, source, original), Some(NameText("x.Z")));
    assert!(derived_of(&symbols).0.is_empty());
    1
}

/// Cross-file names may refer only to bytes actually owned by the cross-file pool.
fn cross_file_names_require_derived_storage() -> usize {
    let source = ByteSlice(b"x.A x.Z");
    let mut symbols = Symbols::default();
    let from_source = intern(&mut symbols, source, Offset(0), Offset(3));
    assert_eq!(name_made_across(&symbols, from_source), None);
    let forged = ryo_relations::domain::relations::types::Name {
        hash: hash_of(ByteSlice(b"x.A")),
        place: ryo_relations::domain::relations::types::PlaceNumber(
            ryo_relations::domain::relations::types::CROSS_PLACE | from_source.0,
        ),
    };
    assert_eq!(place_of(forged, &[], &symbols), None);
    let from_derived = intern_derived(&mut symbols, source, NameText("x.Z"));
    let retained =
        name_made_across(&symbols, from_derived).expect("derived names can leave the pool");
    let alias = intern(&mut symbols, source, Offset(4), Offset(7));
    assert_eq!(alias, from_derived);
    assert_eq!(name_made_across(&symbols, alias), Some(retained));
    let place = place_of(retained, &[], &symbols).expect("cross-file derived place");
    assert_eq!(place.origin, Origin::Derived(CROSS_FILE));
    let start = usize::try_from(place.start.0).expect("fixture offset");
    let end = usize::try_from(place.end.0).expect("fixture offset");
    assert_eq!(&derived_of(&symbols).0[start..end], b"x.Z");
    1
}

fn transported_places_require_the_same_live_symbol_generation() -> usize {
    use crate::domain::symbols::service::{
        clear_symbols, query_has_transport_generation, query_places_mut, transport_permit,
    };
    use ryo_relations::domain::relations::types::NamePlace;

    let mut first = Symbols::default();
    let mut second = Symbols::default();
    let key = transport_permit(&mut first);
    {
        let mut query = QuerySymbols::from(&mut second);
        assert_eq!(
            query_has_transport_generation(&query, &key),
            Generation::Different
        );
        assert!(query_places_mut(&mut query, &key).is_none());
    }
    {
        let mut query = QuerySymbols::from(&mut first);
        assert_eq!(
            query_has_transport_generation(&query, &key),
            Generation::Same
        );
        let places = query_places_mut(&mut query, &key).expect("exact live pool");
        places.push(NamePlace {
            origin: Origin::Derived(WorkerNumber(0)),
            file: FileId(0),
            start: Offset(0),
            end: Offset(1),
        });
        assert_eq!(places.len(), 1);
    }
    clear_symbols(&mut first);
    let mut query = QuerySymbols::from(&mut first);
    assert_eq!(
        query_has_transport_generation(&query, &key),
        Generation::Different
    );
    assert!(query_places_mut(&mut query, &key).is_none());
    1
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("transported_places_require_the_same_live_symbol_generation"),
        word: Word::Succeeds,
        input: || transported_places_require_the_same_live_symbol_generation,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("cross_file_names_require_derived_storage"),
        word: Word::Succeeds,
        input: || cross_file_names_require_derived_storage,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("interleaved_source_and_derived_names_keep_text_and_retention"),
        word: Word::Succeeds,
        input: || interleaved_source_and_derived_names_keep_text_and_retention,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_source_alias_does_not_reclassify_existing_derived_storage"),
        word: Word::Succeeds,
        input: || a_source_alias_does_not_reclassify_existing_derived_storage,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("interleaved_names_compose_and_clear_without_changing_storage"),
        word: Word::Succeeds,
        input: || interleaved_names_compose_and_clear_without_changing_storage,
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
