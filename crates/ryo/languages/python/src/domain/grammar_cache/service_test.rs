//! Budget, content, grammar identity and current-source occurrence regressions.
use super::constants::{KEY_CAPACITY, MEMO_ENTRIES, SCRATCH_CAPACITY};
use super::{
    service,
    types::{DialectName, EntryCount, GrammarMemo, Lookup, MemoKey, Scratch, Taken},
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::format_syntax::{
    interface::FormatSyntax,
    types::{
        FormatDialect, FormatErrorKind, FormatParseError, FormatParseResult, FormatSummary,
        ParsedFormat,
    },
};

struct FirstGrammar;
struct SecondGrammar;

fn bounds_reject_large_keys_and_full_memos() {
    let mut memo = GrammarMemo::default();
    service::grammar::<FirstGrammar, _>(&mut memo, DialectName("first-dialect-v1"));
    service::admit(&mut memo, MemoKey("small".into()), 7_u32);
    assert_eq!(
        service::take(&mut memo, Lookup("small")),
        Some(Taken {
            key: MemoKey("small".into()),
            value: 7
        })
    );
    service::admit(&mut memo, MemoKey("x".repeat(KEY_CAPACITY + 1)), 8);
    assert!(memo.entries.is_empty());
    service::scratch(&mut memo, Scratch("x".repeat(SCRATCH_CAPACITY * 2)));
    assert!(memo.scratch.0.is_empty());
    assert!(memo.scratch.0.capacity() <= SCRATCH_CAPACITY);
}

fn parser_identity_and_exact_content_prevent_stale_reuse() {
    let mut memo = GrammarMemo::default();
    service::grammar::<FirstGrammar, _>(&mut memo, DialectName("first-dialect-v1"));
    service::admit(&mut memo, MemoKey("original".into()), 1_u32);
    assert_eq!(service::take(&mut memo, Lookup("edited")), None);
    service::grammar::<SecondGrammar, _>(&mut memo, DialectName("first-dialect-v1"));
    assert_eq!(service::take(&mut memo, Lookup("original")), None);
    service::admit(&mut memo, MemoKey("dialect".into()), 3_u32);
    service::grammar::<SecondGrammar, _>(&mut memo, DialectName("different-dialect-v1"));
    assert_eq!(service::take(&mut memo, Lookup("dialect")), None);
    for round in 0_u32..100_u32 {
        service::empty(&mut memo);
        for item in 0..100 {
            service::admit(&mut memo, MemoKey(format!("{round}:{item}")), item);
        }
        assert!(memo.len() <= EntryCount(MEMO_ENTRIES));
    }
    service::empty(&mut memo);
    assert!(memo.entries.is_empty());
    assert!(memo.len() <= EntryCount(MEMO_ENTRIES));
}

fn all_four_readers_rebuild_occurrences_after_same_file_edits() {
    use crate::domain::model::types::Declarations;
    use crate::fixtures::reading::{initialize_model, parsed};
    use crate::outbound::{
        format_syntax::adapters::{
            ruff_literal_braceformat::grammar::RuffLiteralBraceFormat,
            ruff_literal_cformat::grammar::RuffLiteralCFormat,
        },
        literal_expression::adapters::ruff_string_annotation::grammar::RuffStringAnnotation,
    };
    use ryo::languages::{self, ByteSlice, Tree};
    let mut model = Declarations::default();
    for (source, spelling, fields) in [
        ("value = '{} %s'\n", "'{} %s'", 1),
        ("prefix = 1\nvalue = '{} %s'\n", "'{} %s'", 1),
        ("value = 'no fields'\n", "'no fields'", 0),
        ("value = '{} %s'\n", "'{} %s'", 1),
    ] {
        let mut file = parsed(source.as_bytes()).expect("source parses");
        let tree = Tree {
            elements: &file.elements,
            source: ByteSlice(file.bytes),
            file: languages::FileId(0),
        };
        initialize_model(&tree, &mut model);
        let start = source.find(spelling).expect("literal in source");
        let decoded = crate::domain::format_parsing::service::prepare::<RuffLiteralCFormat>(
            &tree,
            &model.format_parsing,
            &mut file.symbols,
            crate::domain::semantic_syntax::service::positions(&tree),
            |at, symbols| {
                (crate::domain::semantic_syntax::service::visibility(&tree, &model, at, symbols)
                    == crate::domain::semantic_syntax::types::Visibility::Visible)
                    .into()
            },
        );
        assert_eq!(
            usize::try_from(languages::start_at(tree.elements, decoded.occurrences[0].node).0)
                .expect("source offset fits usize"),
            start
        );
        assert!(
            matches!(&decoded.values[0].result, crate::domain::format_syntax::types::FormatParseResult::Parsed(result) if result.fields.len() == fields)
        );
        drop(decoded);
        let raw = crate::domain::raw_format::service::prepare::<RuffLiteralCFormat>(
            &tree,
            &model.raw_format,
            &mut file.symbols,
            crate::domain::semantic_syntax::service::positions(&tree),
            |at, symbols| {
                (crate::domain::semantic_syntax::service::visibility(&tree, &model, at, symbols)
                    == crate::domain::semantic_syntax::types::Visibility::Visible)
                    .into()
            },
        );
        assert_eq!(
            usize::try_from(languages::start_at(tree.elements, raw.occurrences[0].node).0)
                .expect("source offset fits usize"),
            start
        );
        assert!(
            matches!(&raw.values[0], crate::domain::format_syntax::types::FormatParseResult::Parsed(result) if result.fields.len() == fields)
        );
        drop(raw);
        let brace = crate::domain::brace_format::service::prepare::<RuffLiteralBraceFormat>(
            &tree,
            &model.brace_format,
            &mut file.symbols,
            crate::domain::semantic_syntax::service::positions(&tree),
            |at, symbols| {
                (crate::domain::semantic_syntax::service::visibility(&tree, &model, at, symbols)
                    == crate::domain::semantic_syntax::types::Visibility::Visible)
                    .into()
            },
        );
        assert_eq!(
            usize::try_from(languages::start_at(tree.elements, brace.occurrences[0].node).0)
                .expect("source offset fits usize"),
            start
        );
        assert!(
            matches!(&brace.values[0].result, crate::domain::brace_format::types::BraceFormatResult::Parsed(result) if result.fields.len() == fields)
        );
        drop(brace);
        let literal = crate::domain::literal_expression_parsing::service::prepare::<
            RuffStringAnnotation,
        >(
            &tree,
            &model.literal_expression_parsing,
            &mut file.symbols,
            crate::domain::semantic_syntax::service::positions(&tree),
            |at, symbols| {
                (crate::domain::semantic_syntax::service::visibility(&tree, &model, at, symbols)
                    == crate::domain::semantic_syntax::types::Visibility::Visible)
                    .into()
            },
        );
        assert_eq!(
            usize::try_from(languages::start_at(tree.elements, literal.occurrences[0].node).0)
                .expect("source offset fits usize"),
            start
        );
        assert_eq!(literal.occurrences.len(), 1);
    }
}

fn oversized_current_inputs_do_not_become_reusable_content() {
    use crate::domain::model::types::Declarations;
    use crate::fixtures::reading::{initialize_model, parsed};
    use crate::outbound::{
        format_syntax::adapters::{
            ruff_literal_braceformat::grammar::RuffLiteralBraceFormat,
            ruff_literal_cformat::grammar::RuffLiteralCFormat,
        },
        literal_expression::adapters::ruff_string_annotation::grammar::RuffStringAnnotation,
    };
    use ryo::languages::{self, ByteSlice, Tree};
    let mut model = Declarations::default();
    let large = format!("value = '{}'\n", "x".repeat(KEY_CAPACITY * 2));
    for source in [large.as_str(), "value = 'small'\n"] {
        let mut file = parsed(source.as_bytes()).expect("source parses");
        let tree = Tree {
            elements: &file.elements,
            source: ByteSlice(file.bytes),
            file: languages::FileId(0),
        };
        initialize_model(&tree, &mut model);
        assert!(model.format_parsing.borrow().memo.len() <= EntryCount(MEMO_ENTRIES));
        assert!(model.raw_format.borrow().memo.len() <= EntryCount(MEMO_ENTRIES));
        assert!(model.brace_format.borrow().memo.len() <= EntryCount(MEMO_ENTRIES));
        assert!(model.literal_expression_parsing.borrow().memo.len() <= EntryCount(MEMO_ENTRIES));
        drop(crate::domain::format_parsing::service::prepare::<
            RuffLiteralCFormat,
        >(
            &tree,
            &model.format_parsing,
            &mut file.symbols,
            crate::domain::semantic_syntax::service::positions(&tree),
            |at, symbols| {
                (crate::domain::semantic_syntax::service::visibility(&tree, &model, at, symbols)
                    == crate::domain::semantic_syntax::types::Visibility::Visible)
                    .into()
            },
        ));
        drop(crate::domain::raw_format::service::prepare::<
            RuffLiteralCFormat,
        >(
            &tree,
            &model.raw_format,
            &mut file.symbols,
            crate::domain::semantic_syntax::service::positions(&tree),
            |at, symbols| {
                (crate::domain::semantic_syntax::service::visibility(&tree, &model, at, symbols)
                    == crate::domain::semantic_syntax::types::Visibility::Visible)
                    .into()
            },
        ));
        drop(crate::domain::brace_format::service::prepare::<
            RuffLiteralBraceFormat,
        >(
            &tree,
            &model.brace_format,
            &mut file.symbols,
            crate::domain::semantic_syntax::service::positions(&tree),
            |at, symbols| {
                (crate::domain::semantic_syntax::service::visibility(&tree, &model, at, symbols)
                    == crate::domain::semantic_syntax::types::Visibility::Visible)
                    .into()
            },
        ));
        drop(
            crate::domain::literal_expression_parsing::service::prepare::<RuffStringAnnotation>(
                &tree,
                &model.literal_expression_parsing,
                &mut file.symbols,
                crate::domain::semantic_syntax::service::positions(&tree),
                |at, symbols| {
                    (crate::domain::semantic_syntax::service::visibility(
                        &tree, &model, at, symbols,
                    ) == crate::domain::semantic_syntax::types::Visibility::Visible)
                        .into()
                },
            ),
        );
    }
    // The large active contents have gone; only the current small source is visible.
    assert!(
        model
            .format_parsing
            .borrow()
            .by_input
            .keys()
            .all(|key| key.0.len() < 100)
    );
    assert!(
        model
            .raw_format
            .borrow()
            .by_input
            .keys()
            .all(|key| key.0.len() < 100)
    );
    assert!(
        model
            .brace_format
            .borrow()
            .by_input
            .keys()
            .all(|key| key.0.len() < 100)
    );
    assert!(
        model
            .literal_expression_parsing
            .borrow()
            .by_carrier
            .keys()
            .all(|key| key.input.0.len() < 100)
    );
}

struct ParsedGrammar;
impl FormatSyntax for ParsedGrammar {
    const DECODED_GRAMMAR: crate::domain::format_syntax::types::GrammarIdentity =
        crate::domain::format_syntax::types::GrammarIdentity("test-percent-decoded");
    const RAW_GRAMMAR: crate::domain::format_syntax::types::GrammarIdentity =
        crate::domain::format_syntax::types::GrammarIdentity("test-percent-raw");

    fn parse(
        _: crate::domain::format_syntax::types::FormatInput<'_>,
        _: FormatDialect,
    ) -> FormatParseResult {
        FormatParseResult::Parsed(ParsedFormat {
            fields: Vec::new(),
            summary: FormatSummary::default(),
        })
    }
}
struct RejectedGrammar;
impl FormatSyntax for RejectedGrammar {
    const DECODED_GRAMMAR: crate::domain::format_syntax::types::GrammarIdentity =
        crate::domain::format_syntax::types::GrammarIdentity("test-percent-decoded");
    const RAW_GRAMMAR: crate::domain::format_syntax::types::GrammarIdentity =
        crate::domain::format_syntax::types::GrammarIdentity("test-percent-raw");

    fn parse(
        _: crate::domain::format_syntax::types::FormatInput<'_>,
        _: FormatDialect,
    ) -> FormatParseResult {
        FormatParseResult::Malformed(FormatParseError {
            kind: FormatErrorKind::IncompleteFormat,
            character: None,
            index: crate::domain::format_syntax::types::FormatScalarIndex(0),
        })
    }
}
fn completed_active_results_are_invalidated_when_parser_implementation_changes() {
    use crate::domain::{format_parsing::service::prepare, model::types::Declarations};
    use crate::fixtures::reading::{initialize_model, parsed};
    use ryo::languages::{self, ByteSlice, Symbols, Tree};
    let file = parsed(b"value = 'same input'\n").expect("source parses");
    let tree = Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    let mut model = Declarations::default();
    let mut symbols = Symbols::default();
    initialize_model(&tree, &mut model);
    assert!(matches!(
        prepare::<ParsedGrammar>(
            &tree,
            &model.format_parsing,
            &mut symbols,
            crate::domain::semantic_syntax::service::positions(&tree),
            |at, symbols| (crate::domain::semantic_syntax::service::visibility(
                &tree, &model, at, symbols
            ) == crate::domain::semantic_syntax::types::Visibility::Visible)
                .into()
        )
        .values[0]
            .result,
        FormatParseResult::Parsed(_)
    ));
    assert!(matches!(
        prepare::<RejectedGrammar>(
            &tree,
            &model.format_parsing,
            &mut symbols,
            crate::domain::semantic_syntax::service::positions(&tree),
            |at, symbols| (crate::domain::semantic_syntax::service::visibility(
                &tree, &model, at, symbols
            ) == crate::domain::semantic_syntax::types::Visibility::Visible)
                .into()
        )
        .values[0]
            .result,
        FormatParseResult::Malformed(_)
    ));
}

fn memo_admission_prefers_source_insertion_order_when_the_memo_is_full() {
    use crate::domain::format_syntax::types::{
        CachedFormat, FormatArgument, FormatFieldValue, FormatParseResult, FormatParseState,
        FormatQuantityMode, FormatSummary, ParsedFormat, RetainedFormatIndex,
    };
    for _ in 0_usize..12_usize {
        let mut index =
            RetainedFormatIndex::<FormatParseResult, CachedFormat, FormatParseState>::default();
        let names: Vec<String> = (0..=MEMO_ENTRIES)
            .map(|ordinal| format!("input-{ordinal}"))
            .collect();
        for name in &names {
            assert_eq!(
                service::reserve(&mut index.by_input),
                super::types::Reservation::Granted
            );
            let previous = index.by_input.insert(
                name.clone().into(),
                crate::domain::format_syntax::types::FormatResultIndex(index.values.len()),
            );
            assert!(previous.is_none());
            index.values.push(CachedFormat {
                result: FormatParseResult::Parsed(ParsedFormat {
                    fields: vec![FormatFieldValue {
                        ordinal: crate::domain::format_syntax::types::FormatFieldOrdinal(0),
                        argument: FormatArgument::Mapping(
                            crate::domain::format_syntax::types::FormatMappingKey("x".into()),
                        ),
                        conversion: crate::domain::format_syntax::types::FormatCharacter('s'),
                        flags: Default::default(),
                        width: FormatQuantityMode::None,
                        precision: FormatQuantityMode::None,
                    }],
                    summary: FormatSummary::default(),
                }),
            });
        }
        crate::domain::format_parsing::service::clear(&mut index);
        assert!(index.memo.entries.contains_key(names[0].as_str()));
        assert!(
            index
                .memo
                .entries
                .contains_key(names[MEMO_ENTRIES - 1].as_str())
        );
        assert!(
            !index
                .memo
                .entries
                .contains_key(names[MEMO_ENTRIES].as_str())
        );
        assert!(index.memo.len() <= EntryCount(MEMO_ENTRIES));
    }
}

fn ordered_lookup_releases_large_dense_and_hash_storage_on_reset() {
    let mut map = indexmap::IndexMap::new();
    for ordinal in 0_usize..256_usize {
        assert_eq!(
            service::reserve(&mut map),
            super::types::Reservation::Granted
        );
        let previous = map.insert(ordinal.to_string(), ordinal);
        assert!(previous.is_none());
    }
    assert!(map.capacity() > super::constants::MAP_CAPACITY);
    map.clear();
    service::reset_map(&mut map);
    assert_eq!(map.capacity(), 0);
}

/// Each check below states its own source and asserts its own outcome.
const CASES: &[Case<fn(), (), ()>] = &[
    Case {
        name: CaseName("bounds_reject_large_keys_and_full_memos"),
        word: Word::Succeeds,
        input: || bounds_reject_large_keys_and_full_memos,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("parser_identity_and_exact_content_prevent_stale_reuse"),
        word: Word::Succeeds,
        input: || parser_identity_and_exact_content_prevent_stale_reuse,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("all_four_readers_rebuild_occurrences_after_same_file_edits"),
        word: Word::Succeeds,
        input: || all_four_readers_rebuild_occurrences_after_same_file_edits,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("oversized_current_inputs_do_not_become_reusable_content"),
        word: Word::Succeeds,
        input: || oversized_current_inputs_do_not_become_reusable_content,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(
            "completed_active_results_are_invalidated_when_parser_implementation_changes",
        ),
        word: Word::Succeeds,
        input: || completed_active_results_are_invalidated_when_parser_implementation_changes,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("memo_admission_prefers_source_insertion_order_when_the_memo_is_full"),
        word: Word::Succeeds,
        input: || memo_admission_prefers_source_insertion_order_when_the_memo_is_full,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("ordered_lookup_releases_large_dense_and_hash_storage_on_reset"),
        word: Word::Succeeds,
        input: || ordered_lookup_releases_large_dense_and_hash_storage_on_reset,
        expect: Expect::Value(()),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |check| {
        check();
        Ok(())
    });
}
