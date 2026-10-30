//! The `swc_html_parser` adapter of the syntax port: the port's contract rows run through it, then the cases specific to it.

use super::adapter::SyntaxSwcHtmlParserAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::syntax::service::kind_name;
use crate::outbound::syntax::fixtures::contract::{check, elements_of};

/// html text.
#[derive(Clone, Copy)]
enum Given {
    Empty,
    Text,
    ChangedReference,
    EveryReference,
}
type Input = Given;
/// The elements the adapter read it into, in order, each as its kind.
type Value = &'static str;
type Failure = ();

/// The rows of what is specific to this adapter.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_pinned_document_including_invalid_html_matches_parse5"),
        word: Word::Succeeds,
        input: || Given::EveryReference,
        expect: Expect::Value("135 trees"),
    },
    Case {
        name: CaseName("a_changed_independent_dom_is_a_difference"),
        word: Word::Succeeds,
        input: || Given::ChangedReference,
        expect: Expect::Value("rejected"),
    },
    Case {
        name: CaseName("an_empty_document_contains_its_implied_html_head_and_body"),
        word: Word::Succeeds,
        input: || Given::Empty,
        expect: Expect::Value(
            "source-file document document-mode element name namespace element name namespace element name namespace",
        ),
    },
    Case {
        name: CaseName("native_text_and_lexical_characters_are_both_preserved"),
        word: Word::Succeeds,
        input: || Given::Text,
        expect: Expect::Value(
            "source-file document document-mode element name namespace element name namespace element name namespace text value characters",
        ),
    },
];

fn subject(input: Input) -> Value {
    let text = match input {
        Given::Empty => "",
        Given::Text => "a",
        Given::ChangedReference => return changed_reference(),
        Given::EveryReference => return every_reference(),
    };
    check(&SyntaxSwcHtmlParserAdapter);
    let shown = elements_of(&SyntaxSwcHtmlParserAdapter, text)
        .iter()
        .map(|(kind, _)| kind_name(*kind).0)
        .collect::<Vec<_>>()
        .join(" ");
    Box::leak(shown.into_boxed_str())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}

fn runtime_rows(source: &'static str) -> ryo::languages::contract::OracleRows {
    use crate::domain::relations::collections::relations::{
        syntax_child::item::SyntaxChildItem, syntax_tree::item::SyntaxTreeItem,
    };
    use ryo::languages::{
        Relation,
        contract::{Cell, OracleRows, RelationRows, Want},
    };
    let convert = |cell: &Want| match cell {
        Want::Unlocated => Cell::Unlocated,
        Want::Node(value) => Cell::Node(*value),
        Want::Name(value) => Cell::Name((*value).to_owned()),
        Want::Unknown(value) => Cell::Unknown((*value).to_owned()),
        Want::File(value) => Cell::File((*value).to_owned()),
        Want::Span(from, to) => Cell::Span(*from, *to),
        Want::Number(value) => Cell::Number(ryo::languages::Integer(*value)),
        Want::Flag(value) => Cell::Flag(*value),
    };
    OracleRows(
        [
            (
                Relation::SyntaxTree,
                crate::fixtures::reading::reading::<SyntaxTreeItem>(source),
            ),
            (
                Relation::SyntaxChild,
                crate::fixtures::reading::reading::<SyntaxChildItem>(source),
            ),
        ]
        .into_iter()
        .map(|(relation, reading)| RelationRows {
            relation,
            rows: reading
                .rows
                .iter()
                .map(|row| row.iter().map(convert).collect())
                .collect(),
        })
        .collect(),
    )
}
fn standard() -> serde_json::Value {
    serde_json::from_str(
        &std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/standard")
                .join("parse5.json"),
        )
        .expect("pinned standard dump"),
    )
    .expect("valid pinned reference JSON")
}
fn changed_reference() -> Value {
    let source = "<p title='a'>";
    let rows = runtime_rows(source);
    let standard = standard();
    let Some(expected) = standard.get(source) else {
        return "reference source is absent";
    };
    if !super::fixtures::oracle::against(&rows, expected).is_empty() {
        return "valid independent reference differs from runtime";
    }
    let mut changed = expected.clone();
    changed[2][0][1] = serde_json::Value::String("changed document mode".to_owned());
    if super::fixtures::oracle::against(&rows, &changed).is_empty() {
        "corrupted reference was accepted"
    } else {
        "rejected"
    }
}

fn every_reference() -> Value {
    let standard = standard();
    let Some(documents) = standard.as_object() else {
        return "reference is not a document map";
    };
    let mut failures = Vec::new();
    for (source, expected) in documents {
        let source = Box::leak(source.clone().into_boxed_str());
        let errors = super::fixtures::oracle::against(&runtime_rows(source), expected);
        if !errors.is_empty() {
            failures.push(format!("{source:?}: {}", errors.join("; ")));
        }
    }
    if !failures.is_empty() {
        return Box::leak(
            format!("{} differences: {}", failures.len(), failures.join("\n")).into_boxed_str(),
        );
    }

    Box::leak(format!("{} trees", documents.len()).into_boxed_str())
}
