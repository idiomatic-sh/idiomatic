//! Actual parser output is inspected without a second lexer or inferred token IDs.
use crate::domain::token_links::{
    service::scan,
    types::{Event, LinkKind},
};
use crate::fixtures::reading::parsed;
use ryo::languages::{self, ByteSlice, Tree};

fn edges(source: &str) -> (Vec<(String, String, LinkKind)>, bool) {
    let file = parsed(source.as_bytes()).expect("fixture is accepted or recovered");
    let tree = Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    let mut edges = Vec::new();
    let mut partial = false;
    for event in scan(&tree) {
        match event {
            Event::Incomplete => partial = true,
            Event::Link(link) => {
                let text = |at| {
                    languages::text_of(
                        &file.symbols,
                        tree.source,
                        languages::text_at(tree.elements, at),
                    )
                    .expect("physical token text")
                    .0
                    .to_owned()
                };
                edges.push((text(link.source), text(link.target), link.kind));
            }
        }
    }
    (edges, partial)
}

#[test]
fn immediate_enclosure_excludes_a_closing_tokens_own_pair() {
    let (edges, partial) = edges("f((x, y), [z])\n");
    assert!(!partial);
    let enclosed: Vec<_> = edges
        .iter()
        .filter(|edge| edge.2 == LinkKind::Enclosing)
        .map(|edge| (edge.0.as_str(), edge.1.as_str()))
        .collect();
    assert_eq!(
        enclosed,
        [
            ("(", "("),
            ("x", "("),
            (",", "("),
            ("y", "("),
            (")", "("),
            (",", "("),
            ("[", "("),
            ("z", "["),
            ("]", "(")
        ]
    );
    assert_eq!(
        edges
            .iter()
            .filter(|edge| edge.2 == LinkKind::Matching)
            .count(),
        3
    );
}

#[test]
fn strings_comments_yield_and_commas_have_physical_ownership() {
    let (edges, partial) = edges("def f():\n    return ((yield x), '[(]', # )]\n            y)\n");
    assert!(!partial);
    assert!(
        edges
            .iter()
            .any(|edge| edge.0 == "yield" && edge.1 == "(" && edge.2 == LinkKind::Enclosing)
    );
    assert_eq!(
        edges
            .iter()
            .filter(|edge| edge.2 == LinkKind::Matching)
            .count(),
        3
    );
    assert!(!edges.iter().any(|edge| edge.0 == "[(]"));
}

#[test]
fn unmatched_openers_preserve_prefix_edges_but_not_complete_absence() {
    let (edges, partial) = edges("f((x,\n");
    assert!(partial);
    assert!(
        edges
            .iter()
            .any(|edge| edge.0 == "x" && edge.2 == LinkKind::Enclosing)
    );
    assert!(!edges.iter().any(|edge| edge.2 == LinkKind::Matching));
}

#[test]
fn full_reader_contract_and_incomplete_pairing_remain_observable() {
    use super::TokenLinkItem;
    use crate::domain::relations::collections::relations::fixtures::contract::check;
    use crate::fixtures::reading::reading;
    check::<TokenLinkItem>();
    let complete = reading::<TokenLinkItem>("f((x, y))\n");
    assert!(complete.causes.is_empty(), "{:?}", complete.causes);
    assert!(!complete.rows.is_empty());
    for source in ["f((x,\n", "([)]\n", "f('unterminated\n"] {
        let partial = reading::<TokenLinkItem>(source);
        assert!(
            !partial.causes.is_empty(),
            "{source:?} cannot establish complete absence"
        );
    }
}

#[test]
fn mismatched_closer_has_no_guessed_enclosing_edge() {
    let (edges, partial) = edges("([)]\n");
    assert!(partial);
    assert!(
        !edges
            .iter()
            .any(|edge| edge.0 == ")" && edge.2 == LinkKind::Enclosing)
    );
    assert!(!edges.iter().any(|edge| edge.2 == LinkKind::Matching));
}
