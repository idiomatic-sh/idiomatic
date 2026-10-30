//! The `parameter` item's cases, every row in full, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::ParameterItem;
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
    name: CaseName("each_parameter_says_how_it_takes_its_argument"),
    word: Word::Succeeds,
    input: || "def f(a, /, b, *args, c, d=1, **kw): pass\ng = lambda x, *, y: 0\n",
    expect: Expect::Value(Reading {
        rows: &[
            &[
                want_node(9),
                want_name("positional"),
                want_name("positional-only"),
                want_name("a"),
                want_number(0),
                want_node(1),
                want_file(SHOWN),
                want_span((1, 7), (1, 8)),
            ],
            &[
                want_node(18),
                want_name("positional-or-keyword"),
                want_name("positional-or-keyword"),
                want_name("b"),
                want_number(1),
                want_node(1),
                want_file(SHOWN),
                want_span((1, 13), (1, 14)),
            ],
            &[
                want_node(23),
                want_name("variadic-positional"),
                want_name("var-positional"),
                want_name("args"),
                want_number(2),
                want_node(1),
                want_file(SHOWN),
                want_span((1, 16), (1, 21)),
            ],
            &[
                want_node(30),
                want_name("keyword-only"),
                want_name("keyword-only"),
                want_name("c"),
                want_number(3),
                want_node(1),
                want_file(SHOWN),
                want_span((1, 23), (1, 24)),
            ],
            &[
                want_node(36),
                want_name("keyword-only"),
                want_name("keyword-only"),
                want_name("d"),
                want_number(4),
                want_node(1),
                want_file(SHOWN),
                want_span((1, 26), (1, 27)),
            ],
            &[
                want_node(44),
                want_name("variadic-keyword"),
                want_name("var-keyword"),
                want_name("kw"),
                want_number(5),
                want_node(1),
                want_file(SHOWN),
                want_span((1, 31), (1, 35)),
            ],
            &[
                want_node(65),
                want_name("positional-or-keyword"),
                want_name("positional-or-keyword"),
                want_name("x"),
                want_number(0),
                want_node(60),
                want_file(SHOWN),
                want_span((2, 12), (2, 13)),
            ],
            &[
                want_node(74),
                want_name("keyword-only"),
                want_name("keyword-only"),
                want_name("y"),
                want_number(1),
                want_node(60),
                want_file(SHOWN),
                want_span((2, 18), (2, 19)),
            ],
        ],
        causes: &[],
    }),
}];

fn subject(source: Input) -> Value {
    check::<ParameterItem>();
    reading::<ParameterItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
