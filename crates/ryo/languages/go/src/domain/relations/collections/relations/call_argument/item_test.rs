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
    name: CaseName(
        "a_name_behind_an_address_or_a_dereference_is_passed_on_and_a_conversion_has_no_row",
    ),
    word: Word::Succeeds,
    input: || "func run(s *S, c int) {\n\tforward(s, &c, make(), (*s))\n\t_ = []byte(c)\n}\n",
    expect: Expect::Value(Reading {
        rows: &[
            &[
                want_node(31),
                want_number(0),
                want_node(35),
                want_name("s"),
                want_file(SHOWN),
                want_span((2, 10), (2, 11)),
            ],
            &[
                want_node(31),
                want_number(1),
                want_node(39),
                want_name("c"),
                want_file(SHOWN),
                want_span((2, 13), (2, 15)),
            ],
            &[
                want_node(31),
                want_number(2),
                want_node(45),
                want_name(""),
                want_file(SHOWN),
                want_span((2, 17), (2, 23)),
            ],
            &[
                want_node(31),
                want_number(3),
                want_node(52),
                want_name("s"),
                want_file(SHOWN),
                want_span((2, 25), (2, 29)),
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
