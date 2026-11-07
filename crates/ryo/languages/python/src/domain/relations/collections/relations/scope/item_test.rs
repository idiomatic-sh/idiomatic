//! The `scope` item's cases, every row in full, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::ScopeItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, reading};
use ryo::languages::contract::{Reading, want_file, want_node, want_span};

/// The source the item reads.
type Input = &'static str;
/// What the item writes for it: every row, every column shown, and why it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("the_module_functions_classes_lambdas_and_comprehensions_are_scopes"),
    word: Word::Succeeds,
    input: || "class C:\n    def m(self):\n        return [x for x in (lambda: 0)()]\n",
    expect: Expect::Value(Reading {
        rows: &[
            &[
                want_node(0),
                want_node(0),
                want_file(SHOWN),
                want_span((1, 1), (4, 1)),
            ],
            &[
                want_node(0),
                want_node(1),
                want_file(SHOWN),
                want_span((1, 1), (3, 42)),
            ],
            &[
                want_node(1),
                want_node(9),
                want_file(SHOWN),
                want_span((2, 5), (3, 42)),
            ],
            &[
                want_node(9),
                want_node(27),
                want_file(SHOWN),
                want_span((3, 16), (3, 42)),
            ],
            &[
                want_node(9),
                want_node(42),
                want_file(SHOWN),
                want_span((3, 29), (3, 38)),
            ],
        ],
        causes: &[],
    }),
}];

fn subject(source: Input) -> Value {
    check::<ScopeItem>();
    reading::<ScopeItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
