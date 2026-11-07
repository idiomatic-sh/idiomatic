//! The `static_call` item's cases, every row as its cells, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::StaticCallItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, reading};
use ryo::languages::contract::{Reading, want_file, want_name, want_node, want_span, want_unknown};

/// The source the item reads.
type Input = &'static str;
/// What the item writes for it: every cell of every row, and why it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_call_is_a_row_by_the_dotted_name_it_calls"),
        word: Word::Succeeds,
        input: || "func f() { fmt.Println(); a.b.c(); g() }\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(15),
                    want_node(1),
                    want_name("fmt.Println"),
                    want_file(SHOWN),
                    want_span((1, 12), (1, 23)),
                ],
                &[
                    want_node(27),
                    want_node(1),
                    want_name("a.b.c"),
                    want_file(SHOWN),
                    want_span((1, 27), (1, 32)),
                ],
                &[
                    want_node(43),
                    want_node(1),
                    want_name("g"),
                    want_file(SHOWN),
                    want_span((1, 36), (1, 37)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("any_other_callee_is_dynamic"),
        word: Word::Succeeds,
        input: || "func f() { g()(); xs[0](); []byte(s); func() {}() }\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(15),
                    want_node(1),
                    want_unknown("dynamic-callee"),
                    want_file(SHOWN),
                    want_span((1, 12), (1, 15)),
                ],
                &[
                    want_node(16),
                    want_node(1),
                    want_name("g"),
                    want_file(SHOWN),
                    want_span((1, 12), (1, 13)),
                ],
                &[
                    want_node(26),
                    want_node(1),
                    want_unknown("dynamic-callee"),
                    want_file(SHOWN),
                    want_span((1, 19), (1, 24)),
                ],
                &[
                    want_node(52),
                    want_node(1),
                    want_unknown("dynamic-callee"),
                    want_file(SHOWN),
                    want_span((1, 39), (1, 48)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_function_instantiated_with_types_is_a_dotted_name"),
        word: Word::Succeeds,
        input: || "func f() { m[int, string](x) }\n",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(15),
                want_node(1),
                want_name("m"),
                want_file(SHOWN),
                want_span((1, 12), (1, 26)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_call_is_by_its_dotted_callee_and_any_other_callee_is_dynamic"),
        word: Word::Succeeds,
        input: || "package p\nfunc f() {\n\ta.b()\n\tc()()\n\td[0]()\n\t[]byte(s)\n}\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(20),
                    want_node(6),
                    want_name("a.b"),
                    want_file(SHOWN),
                    want_span((3, 2), (3, 5)),
                ],
                &[
                    want_node(31),
                    want_node(6),
                    want_unknown("dynamic-callee"),
                    want_file(SHOWN),
                    want_span((4, 2), (4, 5)),
                ],
                &[
                    want_node(32),
                    want_node(6),
                    want_name("c"),
                    want_file(SHOWN),
                    want_span((4, 2), (4, 3)),
                ],
                &[
                    want_node(41),
                    want_node(6),
                    want_unknown("dynamic-callee"),
                    want_file(SHOWN),
                    want_span((5, 2), (5, 6)),
                ],
            ],
            causes: &[],
        }),
    },
];

fn subject(source: Input) -> Value {
    check::<StaticCallItem>();
    reading::<StaticCallItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
