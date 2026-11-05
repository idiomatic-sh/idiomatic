//! Demand, representation, reset and unavailable-knowledge contracts.
use super::{
    interface::LiteralExpressionSyntax,
    service::prepare,
    types::{LiteralDialect, LiteralParseOutcome, LiteralParseState},
};
use crate::domain::format_syntax::types::FormatParseState;
use crate::domain::{
    brace_format::service::prepare as brace, format_parsing::service::prepare as decoded,
    model::types::Declarations,
};
use crate::fixtures::reading::{initialize_model, parsed};
use crate::outbound::{
    format_syntax::adapters::{
        ruff_literal_braceformat::RuffLiteralBraceFormat, ruff_literal_cformat::RuffLiteralCFormat,
    },
    literal_expression::ruff_string_annotation::RuffStringAnnotation,
};
use ryo::languages::{self, ByteSlice, Tree};
use std::cell::Cell;

thread_local! {
    /// Relation-specific parser calls in the current isolated test thread.
    static CALLS: Cell<usize> = const { Cell::new(0) };
}
/// Count actual adapter invocations without changing its syntactic decisions.
struct Counting;
impl LiteralExpressionSyntax for Counting {
    fn parse(carrier: &str, dialect: LiteralDialect) -> LiteralParseOutcome {
        CALLS.with(|count| count.set(count.get() + 1));
        RuffStringAnnotation::parse(carrier, dialect)
    }
}

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
    let index = prepare::<Counting>(&tree, &model, &mut source.symbols);
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
    drop(prepare::<Counting>(&tree, &model, &mut source.symbols));
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
        let index = prepare::<Counting>(&tree, &model, &mut source.symbols);
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
    let index = prepare::<Counting>(&tree, &model, &mut source.symbols);
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
    let index = prepare::<Counting>(&tree, &model, &mut source.symbols);
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

#[test]
fn missing_logical_carriers_are_unavailable_and_partial_not_malformed() {
    CALLS.with(|count| count.set(0));
    let source = parsed(b"'T'\n").expect("source");
    let real = Tree {
        elements: &source.elements,
        source: ByteSlice(source.bytes),
        file: languages::FileId(0),
    };
    let mut model = Declarations::default();
    initialize_model(&real, &mut model);
    let unavailable = Tree {
        elements: &source.elements,
        source: ByteSlice(b""),
        file: languages::FileId(0),
    };
    let mut symbols = source.symbols;
    let index = prepare::<Counting>(&unavailable, &model, &mut symbols);
    assert_eq!(index.state, LiteralParseState::Partial);
    assert_eq!(index.occurrences.len(), 1);
    assert!(
        index
            .occurrences
            .iter()
            .all(|row| row.outcome == LiteralParseOutcome::Unavailable)
    );
    CALLS.with(|count| assert_eq!(count.get(), 0));
}

#[test]
fn retained_storage_is_outcomes_occurrences_and_complete_carrier_keys() {
    use std::fmt::Write as _;
    CALLS.with(|count| count.set(0));
    let mut text = String::new();
    for number in 0_u32..256 {
        writeln!(text, "'T{number}'").expect("writing into String is infallible");
    }
    let source_text: &'static str = Box::leak(text.into_boxed_str());
    let mut source = parsed(source_text.as_bytes()).expect("many distinct carriers");
    let tree = Tree {
        elements: &source.elements,
        source: ByteSlice(source.bytes),
        file: languages::FileId(0),
    };
    let mut model = Declarations::default();
    initialize_model(&tree, &mut model);
    let index = prepare::<Counting>(&tree, &model, &mut source.symbols);
    assert_eq!(index.by_carrier.len(), 256);
    assert_eq!(index.occurrences.len(), 256);
    CALLS.with(|count| assert_eq!(count.get(), 256));
    drop(index);
    let large: &'static str = Box::leak(format!("'{}'\n", "T".repeat(65_536)).into_boxed_str());
    let mut large_source = parsed(large.as_bytes()).expect("one bounded large carrier");
    let large_tree = Tree {
        elements: &large_source.elements,
        source: ByteSlice(large_source.bytes),
        file: languages::FileId(0),
    };
    initialize_model(&large_tree, &mut model);
    let large_index = prepare::<Counting>(&large_tree, &model, &mut large_source.symbols);
    assert_eq!(large_index.by_carrier.len(), 1);
    assert_eq!(large_index.occurrences.len(), 1);
    assert!(
        large_index
            .by_carrier
            .keys()
            .all(|(_, carrier)| carrier.len() <= large.len() + 8)
    );
    CALLS.with(|count| assert_eq!(count.get(), 257));
}

#[test]
fn literal_demand_does_not_initialize_other_parser_caches() {
    CALLS.with(|count| count.set(0));
    let mut source = parsed(b"'T'\n").expect("source");
    let tree = Tree {
        elements: &source.elements,
        source: ByteSlice(source.bytes),
        file: languages::FileId(0),
    };
    let mut model = Declarations::default();
    initialize_model(&tree, &mut model);
    drop(prepare::<Counting>(&tree, &model, &mut source.symbols));
    assert_eq!(model.format_parsing.borrow().state, FormatParseState::Fresh);
    assert_eq!(model.raw_format.borrow().state, FormatParseState::Fresh);
    assert_eq!(
        model.brace_format.borrow().state,
        crate::domain::brace_format::types::BraceFormatState::Fresh
    );
    CALLS.with(|count| assert_eq!(count.get(), 1));
}
