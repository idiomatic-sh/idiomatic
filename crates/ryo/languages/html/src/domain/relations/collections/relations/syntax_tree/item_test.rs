//! The `syntax_tree` item's cases, every row as its cells, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::SyntaxTreeItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, reading};
use ryo::languages::contract::{Reading, want_file, want_name, want_node, want_span};

/// The source the item reads.
type Input = &'static str;
/// What the item writes for it: every cell of every row, and why it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName(
        "an_empty_document_has_explicit_implied_elements_and_their_properties_without_locations",
    ),
    word: Word::Succeeds,
    input: || "",
    expect: Expect::Value(Reading {
        rows: &[
            &[
                want_node(0),
                want_name("source-file"),
                want_name(""),
                want_file(SHOWN),
                want_span((1, 1), (1, 1)),
            ],
            &[
                want_node(1),
                want_name("document"),
                want_name("[[7,\"quirks\"]]"),
                want_file(SHOWN),
                want_span((1, 1), (1, 1)),
            ],
            &[
                want_node(2),
                want_name("document-mode"),
                want_name("quirks"),
                want_file(SHOWN),
                ryo::languages::contract::Want::Unlocated,
            ],
            &[
                want_node(3),
                want_name("element"),
                want_name("[[1,\"html\"],[2,\"http://www.w3.org/1999/xhtml\"]]"),
                want_file(SHOWN),
                ryo::languages::contract::Want::Unlocated,
            ],
            &[
                want_node(4),
                want_name("name"),
                want_name("html"),
                want_file(SHOWN),
                ryo::languages::contract::Want::Unlocated,
            ],
            &[
                want_node(5),
                want_name("namespace"),
                want_name("http://www.w3.org/1999/xhtml"),
                want_file(SHOWN),
                ryo::languages::contract::Want::Unlocated,
            ],
            &[
                want_node(6),
                want_name("element"),
                want_name("[[1,\"head\"],[2,\"http://www.w3.org/1999/xhtml\"]]"),
                want_file(SHOWN),
                ryo::languages::contract::Want::Unlocated,
            ],
            &[
                want_node(7),
                want_name("name"),
                want_name("head"),
                want_file(SHOWN),
                ryo::languages::contract::Want::Unlocated,
            ],
            &[
                want_node(8),
                want_name("namespace"),
                want_name("http://www.w3.org/1999/xhtml"),
                want_file(SHOWN),
                ryo::languages::contract::Want::Unlocated,
            ],
            &[
                want_node(9),
                want_name("element"),
                want_name("[[1,\"body\"],[2,\"http://www.w3.org/1999/xhtml\"]]"),
                want_file(SHOWN),
                ryo::languages::contract::Want::Unlocated,
            ],
            &[
                want_node(10),
                want_name("name"),
                want_name("body"),
                want_file(SHOWN),
                ryo::languages::contract::Want::Unlocated,
            ],
            &[
                want_node(11),
                want_name("namespace"),
                want_name("http://www.w3.org/1999/xhtml"),
                want_file(SHOWN),
                ryo::languages::contract::Want::Unlocated,
            ],
        ],
        causes: &[],
    }),
}];

fn subject(source: Input) -> Value {
    check::<SyntaxTreeItem>();
    reading::<SyntaxTreeItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
