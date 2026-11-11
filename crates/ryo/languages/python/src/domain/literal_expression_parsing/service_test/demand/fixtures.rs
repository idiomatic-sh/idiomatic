//! The scenarios of the group: each runs its subject and records what it observed.
use super::super::super::service::prepare;
use super::super::super::types::LiteralParseOutcome;
use super::super::fixtures::{CALLS, Counting};
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

pub(super) fn literal_parsing_is_absent_without_demand_and_caches_both_outcomes() -> String {
    let mut observed: Vec<String> = Vec::new();
    CALLS.with(|count| count.set(0));
    let mut source = parsed(b"'T'\n'T'\n''\n''\n").expect("source");
    let tree = Tree {
        elements: &source.elements,
        source: ByteSlice(source.bytes),
        file: languages::FileId(0),
    };
    let mut model = Declarations::default();
    initialize_model(&tree, &mut model);
    observed.push(format!(
        "{:?}",
        model.literal_expression_parsing.borrow().state
    ));
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
    CALLS.with(|count| observed.push(format!("{:?}", count.get())));
    observed.push(format!(
        "{:?}",
        model.literal_expression_parsing.borrow().state
    ));
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
    CALLS.with(|count| observed.push(format!("{:?}", count.get())));
    observed.push(format!("{:?}", index.state));
    observed.push(format!("{:?}", index.by_carrier.len()));
    observed.push(format!("{:?}", index.occurrences.len()));
    let outcomes = index
        .occurrences
        .iter()
        .map(|row| row.outcome)
        .collect::<Vec<_>>();
    observed.push(format!("{:?}", outcomes));
    let identities = index
        .occurrences
        .iter()
        .map(|row| row.node)
        .collect::<std::collections::HashSet<_>>();
    observed.push(format!("{:?}", identities.len()));
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
    CALLS.with(|count| observed.push(format!("{:?}", count.get())));
    observed.join("|")
}

pub(super) fn reset_discards_old_identities_results_and_key_bytes_but_keeps_capacity() -> String {
    let mut observed: Vec<String> = Vec::new();
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
        observed.push(format!("{:?}", fresh.state));
        observed.push(format!("{}", fresh.occurrences.is_empty()));
        observed.push(format!("{}", fresh.by_carrier.is_empty()));
        observed.push(format!(
            "{}",
            fresh.occurrences.capacity() >= retained_capacity
        ));
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
        observed.push(format!(
            "{:?}",
            index.occurrences.first().map(|row| row.outcome)
        ));
        retained_capacity = index.occurrences.capacity();
        CALLS.with(|count| observed.push(format!("{:?}", count.get())));
    }
    observed.join("|")
}

pub(super) fn equal_decoded_text_keeps_representation_sensitive_parser_modes() -> String {
    let mut observed: Vec<String> = Vec::new();
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
    observed.push(format!("{:?}", index.by_carrier.len()));
    let outcomes = index
        .occurrences
        .iter()
        .map(|row| row.outcome)
        .collect::<Vec<_>>();
    observed.push(format!("{:?}", outcomes));
    CALLS.with(|count| observed.push(format!("{:?}", count.get())));
    observed.join("|")
}

pub(super) fn an_existing_hidden_embedded_success_is_reused_without_new_parsing() -> String {
    let mut observed: Vec<String> = Vec::new();
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
    observed.push(format!("{}", embedded > 0));
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
    observed.push(format!("{:?}", index.occurrences.len()));
    observed.push(format!(
        "{}",
        index
            .occurrences
            .iter()
            .all(|row| row.outcome == LiteralParseOutcome::Parsed)
    ));
    CALLS.with(|calls| observed.push(format!("{:?}", calls.get())));
    observed.push(format!(
        "{:?}",
        languages::contract::element_count(tree.elements)
    ));
    observed.push(format!(
        "{:?}",
        languages::embedded_expressions(tree.elements).count()
    ));
    observed.join("|")
}
