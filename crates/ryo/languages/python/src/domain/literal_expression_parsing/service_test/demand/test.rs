//! Demand, representation, reset and unavailable-knowledge contracts.
use super::super::fixtures::{CALLS, Counting};
use super::super::super::service::prepare;
use super::super::super::types::LiteralParseOutcome;
use super::super::super::types::LiteralParseState;
use crate::domain::brace_format::service::prepare as brace;
use crate::domain::format_parsing::service::prepare as decoded;
use crate::domain::model::types::Declarations;
use crate::fixtures::reading::initialize_model;
use crate::fixtures::reading::parsed;
use crate::outbound::format_syntax::adapters::ruff_literal_braceformat::grammar::RuffLiteralBraceFormat;
use crate::outbound::format_syntax::adapters::ruff_literal_cformat::grammar::RuffLiteralCFormat;
use ryo::languages;
use ryo::languages::ByteSlice;
use ryo::languages::Tree;

#[test]
fn literal_parsing_is_absent_without_demand_and_caches_both_outcomes() {
    CALLS.with(|count| count.set(0));
    let mut source = parsed(b"'T'\n'T'\n''\n''\n").expect("source");
    let tree = Tree {
        elements: &source.elements,
        source: ByteSlice(source.bytes),
        file: languages::FileId(0),
    };
    let mut model = Declarations::default();
    initialize_model(&tree, &mut model);
    assert_eq!(
        model.literal_expression_parsing.borrow().state,
        LiteralParseState::Fresh
    );
    drop(decoded::<RuffLiteralCFormat>(
        &tree,
        &model.format_parsing,
        &mut source.symbols,
        crate::domain::semantic_syntax::service::positions(&tree),
        |at, symbols| {
            (crate::domain::semantic_syntax::service::visibility(&tree, &model, at, symbols)
                == crate::domain::semantic_syntax::types::Visibility::Visible)
                .into()
        },
    ));
    drop(brace::<RuffLiteralBraceFormat>(
        &tree,
        &model.brace_format,
        &mut source.symbols,
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
        &mut source.symbols,
        crate::domain::semantic_syntax::service::positions(&tree),
        |at, symbols| {
            (crate::domain::semantic_syntax::service::visibility(&tree, &model, at, symbols)
                == crate::domain::semantic_syntax::types::Visibility::Visible)
                .into()
        },
    ));
    CALLS.with(|count| assert_eq!(count.get(), 0));
    assert_eq!(
        model.literal_expression_parsing.borrow().state,
        LiteralParseState::Fresh
    );
    let index = prepare::<Counting>(
        &tree,
        &model.literal_expression_parsing,
        &mut source.symbols,
        crate::domain::semantic_syntax::service::positions(&tree),
        |at, symbols| {
            (crate::domain::semantic_syntax::service::visibility(&tree, &model, at, symbols)
                == crate::domain::semantic_syntax::types::Visibility::Visible)
                .into()
        },
    );
    CALLS.with(|count| assert_eq!(count.get(), 2));
    assert_eq!(index.state, LiteralParseState::Complete);
    assert_eq!(index.by_carrier.len(), 2);
    assert_eq!(index.occurrences.len(), 4);
    let outcomes = index
        .occurrences
        .iter()
        .map(|row| row.outcome)
        .collect::<Vec<_>>();
    assert_eq!(
        outcomes,
        [
            LiteralParseOutcome::Parsed,
            LiteralParseOutcome::Parsed,
            LiteralParseOutcome::Malformed,
            LiteralParseOutcome::Malformed
        ]
    );
    let identities = index
        .occurrences
        .iter()
        .map(|row| row.node)
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(
        identities.len(),
        4,
        "equal carriers keep distinct existing occurrence identities"
    );
    drop(index);
    drop(prepare::<Counting>(
        &tree,
        &model.literal_expression_parsing,
        &mut source.symbols,
        crate::domain::semantic_syntax::service::positions(&tree),
        |at, symbols| {
            (crate::domain::semantic_syntax::service::visibility(&tree, &model, at, symbols)
                == crate::domain::semantic_syntax::types::Visibility::Visible)
                .into()
        },
    ));
    CALLS.with(|count| assert_eq!(count.get(), 2));
}

#[test]
fn reset_discards_old_identities_results_and_key_bytes_but_keeps_capacity() {
    CALLS.with(|count| count.set(0));
    let mut model = Declarations::default();
    let mut retained_capacity = 0;
    for (bytes, expected, calls) in [
        (&b"'T'\n"[..], Some(LiteralParseOutcome::Parsed), 1),
        (&b"# moved\n'T'\n"[..], Some(LiteralParseOutcome::Parsed), 1),
        (&b"'T +'\n"[..], Some(LiteralParseOutcome::Malformed), 2),
        (&b"''\n"[..], Some(LiteralParseOutcome::Malformed), 3),
        (&b"pass\n"[..], None, 3),
        (&b"'T'\n"[..], Some(LiteralParseOutcome::Parsed), 4),
    ] {
        let mut source = parsed(bytes).expect("replacement source");
        let tree = Tree {
            elements: &source.elements,
            source: ByteSlice(source.bytes),
            file: languages::FileId(0),
        };
        initialize_model(&tree, &mut model);
        let fresh = model.literal_expression_parsing.borrow();
        assert_eq!(fresh.state, LiteralParseState::Fresh);
        assert!(fresh.occurrences.is_empty());
        assert!(fresh.by_carrier.is_empty());
        assert!(fresh.occurrences.capacity() >= retained_capacity);
        drop(fresh);
        let index = prepare::<Counting>(
            &tree,
            &model.literal_expression_parsing,
            &mut source.symbols,
            crate::domain::semantic_syntax::service::positions(&tree),
            |at, symbols| {
                (crate::domain::semantic_syntax::service::visibility(&tree, &model, at, symbols)
                    == crate::domain::semantic_syntax::types::Visibility::Visible)
                    .into()
            },
        );
        assert_eq!(index.occurrences.first().map(|row| row.outcome), expected);
        retained_capacity = index.occurrences.capacity();
        CALLS.with(|count| assert_eq!(count.get(), calls));
    }
}

#[test]
fn equal_decoded_text_keeps_representation_sensitive_parser_modes() {
    CALLS.with(|count| count.set(0));
    let mut source = parsed(b"'''T\n| U'''\n'T\\n| U'\n('T\\n' '| U')\n").expect("source");
    let tree = Tree {
        elements: &source.elements,
        source: ByteSlice(source.bytes),
        file: languages::FileId(0),
    };
    let mut model = Declarations::default();
    initialize_model(&tree, &mut model);
    let index = prepare::<Counting>(
        &tree,
        &model.literal_expression_parsing,
        &mut source.symbols,
        crate::domain::semantic_syntax::service::positions(&tree),
        |at, symbols| {
            (crate::domain::semantic_syntax::service::visibility(&tree, &model, at, symbols)
                == crate::domain::semantic_syntax::types::Visibility::Visible)
                .into()
        },
    );
    assert_eq!(index.by_carrier.len(), 3);
    let outcomes = index
        .occurrences
        .iter()
        .map(|row| row.outcome)
        .collect::<Vec<_>>();
    assert_eq!(
        outcomes,
        [
            LiteralParseOutcome::Parsed,
            LiteralParseOutcome::Malformed,
            LiteralParseOutcome::Malformed
        ]
    );
    CALLS.with(|count| assert_eq!(count.get(), 3));
}

#[test]
fn an_existing_hidden_embedded_success_is_reused_without_new_parsing() {
    CALLS.with(|count| count.set(0));
    let mut source = parsed(b"from typing import Literal\nx: Literal['T']\n").expect("source");
    let tree = Tree {
        elements: &source.elements,
        source: ByteSlice(source.bytes),
        file: languages::FileId(0),
    };
    let mut model = Declarations::default();
    initialize_model(&tree, &mut model);
    let count = languages::contract::element_count(tree.elements);
    let embedded = languages::embedded_expressions(tree.elements).count();
    assert!(embedded > 0);
    let index = prepare::<Counting>(
        &tree,
        &model.literal_expression_parsing,
        &mut source.symbols,
        crate::domain::semantic_syntax::service::positions(&tree),
        |at, symbols| {
            (crate::domain::semantic_syntax::service::visibility(&tree, &model, at, symbols)
                == crate::domain::semantic_syntax::types::Visibility::Visible)
                .into()
        },
    );
    assert_eq!(index.occurrences.len(), 1);
    assert!(
        index
            .occurrences
            .iter()
            .all(|row| row.outcome == LiteralParseOutcome::Parsed)
    );
    CALLS.with(|calls| assert_eq!(calls.get(), 0));
    assert_eq!(languages::contract::element_count(tree.elements), count);
    assert_eq!(
        languages::embedded_expressions(tree.elements).count(),
        embedded
    );
}
