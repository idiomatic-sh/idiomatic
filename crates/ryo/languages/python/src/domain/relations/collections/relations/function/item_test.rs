//! The `function` item's cases, every row in full, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::FunctionItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, reading};
use ryo::languages::contract::{Reading, want_file, want_name, want_node, want_span};

/// The source the item reads.
type Input = &'static str;
/// What the item writes for it: every row, every column shown, and why it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("every_def_is_a_function_methods_and_nested_ones_too"),
    word: Word::Succeeds,
    input: || "def a():\n    def b(): pass\nclass C:\n    async def m(self): pass\nf = lambda: 0\n",
    expect: Expect::Value(Reading {
        rows: &[
            &[
                want_node(1),
                want_name("a"),
                want_file(SHOWN),
                want_span((1, 1), (2, 18)),
            ],
            &[
                want_node(12),
                want_name("b"),
                want_file(SHOWN),
                want_span((2, 5), (2, 18)),
            ],
            &[
                want_node(34),
                want_name("m"),
                want_file(SHOWN),
                want_span((4, 5), (4, 28)),
            ],
        ],
        causes: &[],
    }),
}];

fn subject(source: Input) -> Value {
    check::<FunctionItem>();
    reading::<FunctionItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
