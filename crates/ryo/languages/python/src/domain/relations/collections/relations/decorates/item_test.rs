//! The `decorates` item's cases, every row in full, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::DecoratesItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, reading};
use ryo::languages::contract::{
    Reading, want_file, want_name, want_node, want_number, want_span, want_unknown,
};

/// The source the item reads.
type Input = &'static str;
/// What the item writes for it: every row, every column shown, and why it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("decorators_name_what_they_write_or_call"),
    word: Word::Succeeds,
    input: || "@a.b\n@c(1)\n@handlers[0]\ndef f(): pass\n",
    expect: Expect::Value(Reading {
        rows: &[
            &[
                want_node(2),
                want_node(1),
                want_number(0),
                want_name("a.b"),
                want_name(""),
                want_file(SHOWN),
                want_span((1, 1), (1, 5)),
            ],
            &[
                want_node(11),
                want_node(1),
                want_number(1),
                want_name("c"),
                want_name("1"),
                want_file(SHOWN),
                want_span((2, 1), (2, 6)),
            ],
            &[
                want_node(22),
                want_node(1),
                want_number(2),
                want_unknown("dynamic-decorator"),
                want_name(""),
                want_file(SHOWN),
                want_span((3, 1), (3, 13)),
            ],
        ],
        causes: &[],
    }),
}];

fn subject(source: Input) -> Value {
    check::<DecoratesItem>();
    reading::<DecoratesItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
