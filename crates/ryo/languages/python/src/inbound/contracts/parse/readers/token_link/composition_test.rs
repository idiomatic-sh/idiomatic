//! Actual parser output is inspected without a second lexer or inferred token IDs.
use crate::cases::{Case, CaseName, Expect, Word, run};
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
    let stack = std::cell::RefCell::default();
    for event in scan(&tree, stack.borrow_mut()) {
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

fn full_reader_contract_and_incomplete_pairing_remain_observable() {
    use super::composition::TokenLinkItem;
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

/// The cases of this file, one variant each.
#[derive(Clone, Copy)]
enum Body {
    /// The case `immediate_enclosure_excludes_a_closing_tokens_own_pair`.
    ImmediateEnclosureExcludesAClosingTokensOwnPair,
    /// The case `strings_comments_yield_and_commas_have_physical_ownership`.
    StringsCommentsYieldAndCommasHavePhysicalOwnership,
    /// The case `unmatched_openers_preserve_prefix_edges_but_not_complete_absence`.
    UnmatchedOpenersPreservePrefixEdgesButNotCompleteAbsence,
    /// The case `full_reader_contract_and_incomplete_pairing_remain_observable`.
    FullReaderContractAndIncompletePairingRemainObservable,
    /// The case `mismatched_closer_has_no_guessed_enclosing_edge`.
    MismatchedCloserHasNoGuessedEnclosingEdge,
}

type Input = Body;
type Value = ();
type Failure = ();

/// The rows of the cases whose bodies assert for themselves.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("immediate_enclosure_excludes_a_closing_tokens_own_pair"),
        word: Word::Succeeds,
        input: || Body::ImmediateEnclosureExcludesAClosingTokensOwnPair,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("strings_comments_yield_and_commas_have_physical_ownership"),
        word: Word::Succeeds,
        input: || Body::StringsCommentsYieldAndCommasHavePhysicalOwnership,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("unmatched_openers_preserve_prefix_edges_but_not_complete_absence"),
        word: Word::Succeeds,
        input: || Body::UnmatchedOpenersPreservePrefixEdgesButNotCompleteAbsence,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("full_reader_contract_and_incomplete_pairing_remain_observable"),
        word: Word::Succeeds,
        input: || Body::FullReaderContractAndIncompletePairingRemainObservable,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("mismatched_closer_has_no_guessed_enclosing_edge"),
        word: Word::Succeeds,
        input: || Body::MismatchedCloserHasNoGuessedEnclosingEdge,
        expect: Expect::Value(()),
    },
];

/// Runs the body of one case.
fn subject(given: Input) -> Result<Value, Failure> {
    match given {
        Body::ImmediateEnclosureExcludesAClosingTokensOwnPair => {
            immediate_enclosure_excludes_a_closing_tokens_own_pair()
        }
        Body::StringsCommentsYieldAndCommasHavePhysicalOwnership => {
            strings_comments_yield_and_commas_have_physical_ownership()
        }
        Body::UnmatchedOpenersPreservePrefixEdgesButNotCompleteAbsence => {
            unmatched_openers_preserve_prefix_edges_but_not_complete_absence()
        }
        Body::FullReaderContractAndIncompletePairingRemainObservable => {
            full_reader_contract_and_incomplete_pairing_remain_observable()
        }
        Body::MismatchedCloserHasNoGuessedEnclosingEdge => {
            mismatched_closer_has_no_guessed_enclosing_edge()
        }
    }
    Ok(())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
