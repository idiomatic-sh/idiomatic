//! Budget, content, grammar identity and current-source occurrence regressions.
use super::{service, types::Memo};

struct FirstGrammar;
struct SecondGrammar;

#[test]
fn byte_budget_rejects_large_keys_and_owned_results() {
    let mut memo = Memo::default();
    service::grammar::<FirstGrammar, _>(&mut memo, "first-dialect-v1");
    service::admit(&mut memo, "small".into(), 7_u32, 0);
    assert_eq!(service::take(&mut memo, "small"), Some(("small".into(), 7)));
    service::admit(&mut memo, "x".repeat(service::BUDGET + 1), 8, 0);
    service::admit(&mut memo, "small".into(), 9, service::BUDGET);
    assert!(memo.entries.is_empty());
    assert!(service::retained(&memo) <= service::MEMO_BUDGET);
    service::scratch(&mut memo, "x".repeat(service::BUDGET * 2));
    assert!(memo.scratch.is_empty());
    assert!(memo.scratch.capacity() <= service::ARENA_BUDGET);
}

#[test]
fn parser_identity_and_exact_content_prevent_stale_reuse() {
    let mut memo = Memo::default();
    service::grammar::<FirstGrammar, _>(&mut memo, "first-dialect-v1");
    service::admit(&mut memo, "original".into(), 1_u32, 0);
    assert_eq!(service::take(&mut memo, "edited"), None);
    service::grammar::<SecondGrammar, _>(&mut memo, "first-dialect-v1");
    assert_eq!(service::take(&mut memo, "original"), None);
    service::admit(&mut memo, "dialect".into(), 3_u32, 0);
    service::grammar::<SecondGrammar, _>(&mut memo, "different-dialect-v1");
    assert_eq!(service::take(&mut memo, "dialect"), None);
    for round in 0_u32..100_u32 {
        service::empty(&mut memo);
        for item in 0..100 {
            service::admit(&mut memo, format!("{round}:{item}"), item, 0);
        }
        assert!(service::retained(&memo) <= service::MEMO_BUDGET);
    }
    service::empty(&mut memo);
    assert!(memo.entries.is_empty());
    assert!(service::retained(&memo) <= service::MEMO_BUDGET);
}

#[test]
fn all_four_readers_rebuild_occurrences_after_same_file_edits() {
    use crate::domain::model::types::Declarations;
    use crate::fixtures::reading::{initialize_model, parsed};
    use crate::outbound::{
        format_syntax::adapters::{
            ruff_literal_braceformat::RuffLiteralBraceFormat,
            ruff_literal_cformat::RuffLiteralCFormat,
        },
        literal_expression::ruff_string_annotation::RuffStringAnnotation,
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
            &model,
            &mut file.symbols,
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
            &model,
            &mut file.symbols,
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
            &model,
            &mut file.symbols,
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
        >(&tree, &model, &mut file.symbols);
        assert_eq!(
            usize::try_from(languages::start_at(tree.elements, literal.occurrences[0].node).0)
                .expect("source offset fits usize"),
            start
        );
        assert_eq!(literal.occurrences.len(), 1);
    }
}

#[test]
fn oversized_current_inputs_do_not_become_reusable_content() {
    use crate::domain::model::types::Declarations;
    use crate::fixtures::reading::{initialize_model, parsed};
    use crate::outbound::{
        format_syntax::adapters::{
            ruff_literal_braceformat::RuffLiteralBraceFormat,
            ruff_literal_cformat::RuffLiteralCFormat,
        },
        literal_expression::ruff_string_annotation::RuffStringAnnotation,
    };
    use ryo::languages::{self, ByteSlice, Tree};
    let mut model = Declarations::default();
    let large = format!("value = '{}'\n", "x".repeat(service::BUDGET * 2));
    for source in [large.as_str(), "value = 'small'\n"] {
        let mut file = parsed(source.as_bytes()).expect("source parses");
        let tree = Tree {
            elements: &file.elements,
            source: ByteSlice(file.bytes),
            file: languages::FileId(0),
        };
        initialize_model(&tree, &mut model);
        assert!(service::retained(&model.format_parsing.borrow().memo) <= service::BUDGET);
        assert!(service::retained(&model.raw_format.borrow().memo) <= service::BUDGET);
        assert!(service::retained(&model.brace_format.borrow().memo) <= service::BUDGET);
        assert!(
            service::retained(&model.literal_expression_parsing.borrow().memo) <= service::BUDGET
        );
        drop(crate::domain::format_parsing::service::prepare::<
            RuffLiteralCFormat,
        >(&tree, &model, &mut file.symbols));
        drop(crate::domain::raw_format::service::prepare::<
            RuffLiteralCFormat,
        >(&tree, &model, &mut file.symbols));
        drop(crate::domain::brace_format::service::prepare::<
            RuffLiteralBraceFormat,
        >(&tree, &model, &mut file.symbols));
        drop(
            crate::domain::literal_expression_parsing::service::prepare::<RuffStringAnnotation>(
                &tree,
                &model,
                &mut file.symbols,
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
            .all(|(_, key)| key.len() < 100)
    );
}

#[test]
fn completed_active_results_are_invalidated_when_parser_implementation_changes() {
    use crate::domain::{
        format_parsing::service::prepare,
        format_syntax::{
            interface::FormatSyntax,
            types::{
                FormatDialect, FormatErrorKind, FormatParseError, FormatParseResult, FormatSummary,
                ParsedFormat,
            },
        },
        model::types::Declarations,
    };
    use crate::fixtures::reading::{initialize_model, parsed};
    use ryo::languages::{self, ByteSlice, Symbols, Tree};
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
        prepare::<ParsedGrammar>(&tree, &model, &mut symbols).values[0].result,
        FormatParseResult::Parsed(_)
    ));
    assert!(matches!(
        prepare::<RejectedGrammar>(&tree, &model, &mut symbols).values[0].result,
        FormatParseResult::Malformed(_)
    ));
}

#[test]
fn memo_admission_prefers_source_insertion_order_when_budget_is_full() {
    use crate::domain::format_syntax::types::{
        CachedFormat, FormatArgument, FormatFieldValue, FormatParseIndex, FormatParseResult,
        FormatQuantityMode, FormatSummary, ParsedFormat,
    };
    for _ in 0_usize..12_usize {
        let mut index = FormatParseIndex::default();
        for name in ["first", "second", "third"] {
            assert!(service::reserve(&mut index.by_input));
            let previous = index.by_input.insert(
                name.to_owned().into(),
                crate::domain::format_syntax::types::FormatResultIndex(index.values.len()),
            );
            assert!(previous.is_none());
            index.values.push(CachedFormat {
                result: FormatParseResult::Parsed(ParsedFormat {
                    fields: vec![FormatFieldValue {
                        ordinal: crate::domain::format_syntax::types::FormatFieldOrdinal(0),
                        argument: FormatArgument::Mapping(
                            crate::domain::format_syntax::types::FormatMappingKey("x".repeat(3000)),
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
        assert!(index.memo.entries.contains_key("first"));
        assert!(index.memo.entries.contains_key("second"));
        assert!(!index.memo.entries.contains_key("third"));
        assert!(service::retained(&index.memo) <= service::MEMO_BUDGET);
        assert!(service::ordered_bytes(&index.by_input) <= service::ARENA_BUDGET);
    }
}

#[test]
fn ordered_lookup_releases_large_dense_and_hash_storage_on_reset() {
    let mut map = indexmap::IndexMap::new();
    for ordinal in 0_usize..256_usize {
        assert!(service::reserve(&mut map));
        let previous = map.insert(ordinal.to_string(), ordinal);
        assert!(previous.is_none());
    }
    assert!(service::ordered_bytes(&map) > service::ARENA_BUDGET);
    map.clear();
    service::reset_map(&mut map);
    assert_eq!(service::ordered_bytes(&map), 0);
    assert_eq!(map.capacity(), 0);
}
