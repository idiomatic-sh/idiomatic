//! Unavailable carriers, retained storage and independence from the other parser caches.
use super::super::service::prepare;
use super::super::types::LiteralParseOutcome;
use super::super::types::LiteralParseState;
use super::{CALLS, Counting};
use crate::cases::Case;
use crate::cases::CaseName;
use crate::cases::Expect;
use crate::cases::Word;
use crate::cases::run;
use crate::domain::format_syntax::types::FormatParseState;
use crate::domain::model::types::Declarations;
use crate::fixtures::reading::initialize_model;
use crate::fixtures::reading::parsed;
use ryo::languages;
use ryo::languages::ByteSlice;
use ryo::languages::Tree;

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
    let index = prepare::<Counting>(
        &unavailable,
        &model.literal_expression_parsing,
        &mut symbols,
        crate::domain::semantic_syntax::service::positions(&unavailable),
        |at, symbols| {
            (crate::domain::semantic_syntax::service::visibility(&unavailable, &model, at, symbols)
                == crate::domain::semantic_syntax::types::Visibility::Visible)
                .into()
        },
    );
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
    let large_index = prepare::<Counting>(
        &large_tree,
        &model.literal_expression_parsing,
        &mut large_source.symbols,
        crate::domain::semantic_syntax::service::positions(&large_tree),
        |at, symbols| {
            (crate::domain::semantic_syntax::service::visibility(&large_tree, &model, at, symbols)
                == crate::domain::semantic_syntax::types::Visibility::Visible)
                .into()
        },
    );
    assert_eq!(large_index.by_carrier.len(), 1);
    assert_eq!(large_index.occurrences.len(), 1);
    assert!(
        large_index
            .by_carrier
            .keys()
            .all(|key| key.input.0.len() <= large.len() + 8)
    );
    CALLS.with(|count| assert_eq!(count.get(), 257));
}

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
    assert_eq!(model.format_parsing.borrow().state, FormatParseState::Fresh);
    assert_eq!(model.raw_format.borrow().state, FormatParseState::Fresh);
    assert_eq!(
        model.brace_format.borrow().state,
        crate::domain::brace_format::types::BraceFormatState::Fresh
    );
    CALLS.with(|count| assert_eq!(count.get(), 1));
}

/// Each check below states its own source and asserts its own outcome.
const CASES: &[Case<fn(), (), ()>] = &[
    Case {
        name: CaseName("missing_logical_carriers_are_unavailable_and_partial_not_malformed"),
        word: Word::Succeeds,
        input: || missing_logical_carriers_are_unavailable_and_partial_not_malformed,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("retained_storage_is_outcomes_occurrences_and_complete_carrier_keys"),
        word: Word::Succeeds,
        input: || retained_storage_is_outcomes_occurrences_and_complete_carrier_keys,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("literal_demand_does_not_initialize_other_parser_caches"),
        word: Word::Succeeds,
        input: || literal_demand_does_not_initialize_other_parser_caches,
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
