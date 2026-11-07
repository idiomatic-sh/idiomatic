//! The `static_call` item's cases, every row in full, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::StaticCallItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, reading};
use ryo::languages::contract::{Reading, want_file, want_name, want_node, want_span, want_unknown};

/// The source the item reads.
type Input = &'static str;
/// What the item writes for it: every row, every column shown, and why it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_call_names_its_dotted_callee_or_is_unknown"),
    word: Word::Succeeds,
    input: || "os.path.join(a)\nhandlers[0]()\nf()()\n",
    expect: Expect::Value(Reading {
        rows: &[
            &[
                want_node(2),
                want_node(0),
                want_name("os.path.join"),
                want_file(SHOWN),
                want_span((1, 1), (1, 13)),
            ],
            &[
                want_node(20),
                want_node(0),
                want_unknown("dynamic-callee"),
                want_file(SHOWN),
                want_span((2, 1), (2, 12)),
            ],
            &[
                want_node(33),
                want_node(0),
                want_unknown("dynamic-callee"),
                want_file(SHOWN),
                want_span((3, 1), (3, 4)),
            ],
            &[
                want_node(34),
                want_node(0),
                want_name("f"),
                want_file(SHOWN),
                want_span((3, 1), (3, 2)),
            ],
        ],
        causes: &[],
    }),
}];

fn subject(source: Input) -> Value {
    check::<StaticCallItem>();
    reading::<StaticCallItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
