//! The `attribute_path` item's cases: every path an attribute writes, as the name it is written as.

use super::super::fixtures::contract::check;
use super::item::AttributePathItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, reading};
use ryo::languages::contract::{Reading, want_file, want_name, want_node, want_span};

/// The source the item reads.
type Input = &'static str;
/// What the item writes for it: every cell of every row, and the causes for which it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_meta_path_and_the_paths_of_its_token_tree"),
    word: Word::Succeeds,
    input: || "#[rustfmt /* c */ ::skip(a :: /* d */ r#b, ::c)]\nfn f() {}",
    expect: Expect::Value(Reading {
        rows: &[
            &[
                want_node(6),
                want_name("rustfmt::skip"),
                want_file(SHOWN),
                want_span((1, 3), (1, 25)),
            ],
            &[
                want_node(20),
                want_name("a::r#b"),
                want_file(SHOWN),
                want_span((1, 26), (1, 42)),
            ],
            &[
                want_node(30),
                want_name("::c"),
                want_file(SHOWN),
                want_span((1, 44), (1, 47)),
            ],
        ],
        causes: &[],
    }),
}];

fn subject(source: Input) -> Value {
    check::<AttributePathItem>();
    reading::<AttributePathItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
