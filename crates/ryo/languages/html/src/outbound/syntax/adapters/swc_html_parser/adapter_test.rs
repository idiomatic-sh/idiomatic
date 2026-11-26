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

/// The rows of the syntax tree and the syntax child relations of one source, as the engine projects them from the node table the
/// adapter walks it into: a node of a spelled kind has text when it is a token or has no children.
fn runtime_rows(source: &'static str) -> ryo::languages::contract::OracleRows {
    use crate::domain::syntax::service::{kind_name, kind_of};
    use crate::outbound::syntax::interface::SyntaxPort as _;
    use crate::outbound::syntax::types::SourceText;
    use ryo::languages::{
        Count, Integer, NodeTable, Position, Relation,
        contract::{Cell, OracleRows, RelationRows},
        node_table,
    };
    let mut table = NodeTable::default();
    let mut builder = node_table::builder(&mut table, Count(64));
    SyntaxSwcHtmlParserAdapter.walk(SourceText(source.as_bytes()), ryo::languages::SourcePath("a.html"), &mut builder);
    let table = node_table::finished(&mut builder);
    let count = u32::try_from(node_table::element_count(table).0).unwrap_or(u32::MAX);
    let mut tree = Vec::new();
    let mut child = Vec::new();
    for id in (0..count).map(Position) {
        let kind = kind_of(node_table::kind_of(table, id));
        let spelled = <crate::domain::file_view::types::HtmlWalk<crate::inbound::contracts::walk::contract::WalkPorts> as ryo::languages::Fillers>::spells(crate::domain::syntax::service::kind_bits(kind))
            && (node_table::is_token(table, id) || node_table::children_of(table, id).is_empty());
        let text = if spelled {
            node_table::decoded_of(table, id).map(|text| text.0.to_owned()).unwrap_or_else(|| {
                let span = node_table::span_of(table, id);
                source
                    .get(usize::try_from(span.start.0).unwrap_or(0)..usize::try_from(span.end.0).unwrap_or(0))
                    .unwrap_or_default()
                    .to_owned()
            })
        } else {
            String::new()
        };
        tree.push(vec![Cell::Node(id.0), Cell::Name(kind_name(kind).0.to_owned()), Cell::Name(text)]);
        let parent = node_table::parent_of(table, id);
        if parent != id {
            child.push(vec![
                Cell::Node(parent.0),
                Cell::Node(id.0),
                Cell::Number(Integer(i64::from(node_table::ordinal_of(table, id).0))),
            ]);
        }
    }
    OracleRows(vec![
        RelationRows { relation: Relation::SyntaxTree, rows: tree },
        RelationRows { relation: Relation::SyntaxChild, rows: child },
    ])
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
