//! The four readers' resets of one source's occurrences and of oversized inputs.
use super::super::super::constants::KEY_CAPACITY;
use super::super::super::constants::MEMO_ENTRIES;
use super::super::super::service;
use super::super::super::types::EntryCount;

#[test]
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

#[test]
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
        assert!(service::len(&model.format_parsing.borrow().memo) <= EntryCount(MEMO_ENTRIES));
        assert!(service::len(&model.raw_format.borrow().memo) <= EntryCount(MEMO_ENTRIES));
        assert!(service::len(&model.brace_format.borrow().memo) <= EntryCount(MEMO_ENTRIES));
        assert!(
            service::len(&model.literal_expression_parsing.borrow().memo)
                <= EntryCount(MEMO_ENTRIES)
        );
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
