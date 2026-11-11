//! The scenarios of the group: each runs its subject and records what it observed.
use super::super::super::service::prepare;
use super::super::super::types::LiteralParseOutcome;
use super::super::fixtures::{CALLS, Counting};
use crate::domain::model::types::Declarations;
use crate::fixtures::reading::initialize_model;
use crate::fixtures::reading::parsed;
use ryo::languages;
use ryo::languages::ByteSlice;
use ryo::languages::Tree;

pub(super) fn missing_logical_carriers_are_unavailable_and_partial_not_malformed() -> String {
    let mut observed: Vec<String> = Vec::new();
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
    observed.push(format!("{:?}", index.state));
    observed.push(format!("{:?}", index.occurrences.len()));
    observed.push(format!(
        "{}",
        index
            .occurrences
            .iter()
            .all(|row| row.outcome == LiteralParseOutcome::Unavailable)
    ));
    CALLS.with(|count| observed.push(format!("{:?}", count.get())));
    observed.join("|")
}

pub(super) fn retained_storage_is_outcomes_occurrences_and_complete_carrier_keys() -> String {
    let mut observed: Vec<String> = Vec::new();
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
    observed.push(format!("{:?}", index.by_carrier.len()));
    observed.push(format!("{:?}", index.occurrences.len()));
    CALLS.with(|count| observed.push(format!("{:?}", count.get())));
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
    observed.push(format!("{:?}", large_index.by_carrier.len()));
    observed.push(format!("{:?}", large_index.occurrences.len()));
    observed.push(format!(
        "{}",
        large_index
            .by_carrier
            .keys()
            .all(|key| key.input.0.len() <= large.len() + 8)
    ));
    CALLS.with(|count| observed.push(format!("{:?}", count.get())));
    observed.join("|")
}

pub(super) fn literal_demand_does_not_initialize_other_parser_caches() -> String {
    let mut observed: Vec<String> = Vec::new();
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
    observed.push(format!("{:?}", model.format_parsing.borrow().state));
    observed.push(format!("{:?}", model.raw_format.borrow().state));
    observed.push(format!("{:?}", model.brace_format.borrow().state));
    CALLS.with(|count| observed.push(format!("{:?}", count.get())));
    observed.join("|")
}
