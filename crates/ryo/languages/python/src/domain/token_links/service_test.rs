//! The `token_links` service's cases: the next-token, matching-delimiter and enclosing-delimiter links between the tokens of a parsed
//! source, and the one incomplete event when pairing is lost.

use super::service::{link_kind, scan};
use super::types::Event;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::syntax::service::written;
use crate::fixtures::reading::parsed;
use ryo::languages::{self, ByteSlice, Tree};

/// A source.
type Input = &'static str;
/// Every event in scan order, `kind source>target` or `incomplete`, joined by `|`.
type Value = &'static str;
/// A source the parser refuses.
type Failure = ();

/// The rows of this concept's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("consecutive_tokens_link_to_the_next"),
        word: Word::Succeeds,
        input: || "a = b\n",
        expect: Expect::Value(
            "next-token \"a\">\"=\"|next-token \"=\">\"b\"|next-token \"b\">\"\\n\"",
        ),
    },
    Case {
        name: CaseName("a_parenthesis_pair_matches_and_encloses_its_tokens"),
        word: Word::Succeeds,
        input: || "f(x)\n",
        expect: Expect::Value(
            "next-token \"f\">\"(\"|next-token \"(\">\"x\"|enclosing-delimiter \"x\">\"(\"|next-token \"x\">\")\"|matching-delimiter \"(\">\")\"|next-token \")\">\"\\n\"",
        ),
    },
    Case {
        name: CaseName("nested_brackets_enclose_in_nesting_order"),
        word: Word::Succeeds,
        input: || "[(1)]\n",
        expect: Expect::Value(
            "next-token \"[\">\"(\"|enclosing-delimiter \"(\">\"[\"|next-token \"(\">\"1\"|enclosing-delimiter \"1\">\"(\"|next-token \"1\">\")\"|matching-delimiter \"(\">\")\"|enclosing-delimiter \")\">\"[\"|next-token \")\">\"]\"|matching-delimiter \"[\">\"]\"|next-token \"]\">\"\\n\"",
        ),
    },
    Case {
        name: CaseName("an_unclosed_bracket_loses_pairing_once"),
        word: Word::Succeeds,
        input: || "f(\n",
        expect: Expect::Value(
            "next-token \"f\">\"(\"|next-token \"(\">\"\\n\"|enclosing-delimiter \"\\n\">\"(\"|incomplete",
        ),
    },
];

/// Scans the source's links.
fn subject(source: Input) -> Result<Value, Failure> {
    let file = parsed(source.as_bytes()).map_err(|_| ())?;
    let tree = Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    let stack = std::cell::RefCell::default();
    let texts: Vec<String> = scan(&tree, stack.borrow_mut())
        .map(|event| match event {
            Event::Link(link) => format!(
                "{} {:?}>{:?}",
                link_kind(link.kind).0,
                written(&tree, link.source).0,
                written(&tree, link.target).0
            ),
            Event::Incomplete => "incomplete".to_owned(),
        })
        .collect();
    Ok(Box::leak(texts.join("|").into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
