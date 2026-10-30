//! The `call_argument` item's cases, every row in full, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::CallArgumentItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, reading};
use ryo::languages::contract::{Reading, want_file, want_name, want_node, want_number, want_span};

/// The source the item reads.
type Input = &'static str;
/// What the item writes for it: every row, every column shown, and why it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_name_behind_a_star_is_passed_on_a_call_is_not_and_a_keyword_gives_its_value"),
    word: Word::Succeeds,
    input: || "def run(s, c):\n    forward(s, *c, make(), key=s)\n    obj.go(c, 1)\n",
    expect: Expect::Value(Reading {
        rows: &[
            &[
                want_node(23),
                want_number(0),
                want_node(28),
                want_name("s"),
                want_file(SHOWN),
                want_span((2, 13), (2, 14)),
            ],
            &[
                want_node(23),
                want_number(1),
                want_node(32),
                want_name("c"),
                want_file(SHOWN),
                want_span((2, 16), (2, 18)),
            ],
            &[
                want_node(23),
                want_number(2),
                want_node(38),
                want_name(""),
                want_file(SHOWN),
                want_span((2, 20), (2, 26)),
            ],
            &[
                want_node(23),
                want_number(3),
                want_node(50),
                want_name("s"),
                want_file(SHOWN),
                want_span((2, 32), (2, 33)),
            ],
            &[
                want_node(56),
                want_number(0),
                want_node(65),
                want_name("c"),
                want_file(SHOWN),
                want_span((3, 12), (3, 13)),
            ],
            &[
                want_node(56),
                want_number(1),
                want_node(69),
                want_name(""),
                want_file(SHOWN),
                want_span((3, 15), (3, 16)),
            ],
        ],
        causes: &[],
    }),
}];

fn subject(source: Input) -> Value {
    check::<CallArgumentItem>();
    reading::<CallArgumentItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
