//! Cases for TypeScript `pattern_child` rows, checked against the shared reader contract.
use super::super::fixtures::contract::check;
use super::item::PatternChildItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::reading;
use ryo::languages::contract::{Reading, want_file, want_name, want_node, want_number};
type Input = &'static str;
type Value = Reading;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_range_has_its_ends"),
        word: Word::Succeeds,
        input: || "/[a-z]/;",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(5),
                    want_node(6),
                    want_number(0),
                    want_name("body"),
                    want_file("item.ts"),
                ],
                &[
                    want_node(6),
                    want_node(7),
                    want_number(0),
                    want_name("body"),
                    want_file("item.ts"),
                ],
                &[
                    want_node(7),
                    want_node(8),
                    want_number(0),
                    want_name("body"),
                    want_file("item.ts"),
                ],
                &[
                    want_node(8),
                    want_node(9),
                    want_number(0),
                    want_name("body"),
                    want_file("item.ts"),
                ],
                &[
                    want_node(9),
                    want_node(10),
                    want_number(0),
                    want_name("min"),
                    want_file("item.ts"),
                ],
                &[
                    want_node(9),
                    want_node(11),
                    want_number(1),
                    want_name("max"),
                    want_file("item.ts"),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("alternatives_are_ordered"),
        word: Word::Succeeds,
        input: || "/a|b/;",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(5),
                    want_node(6),
                    want_number(0),
                    want_name("body"),
                    want_file("item.ts"),
                ],
                &[
                    want_node(6),
                    want_node(7),
                    want_number(0),
                    want_name("body"),
                    want_file("item.ts"),
                ],
                &[
                    want_node(6),
                    want_node(9),
                    want_number(1),
                    want_name("body"),
                    want_file("item.ts"),
                ],
                &[
                    want_node(7),
                    want_node(8),
                    want_number(0),
                    want_name("body"),
                    want_file("item.ts"),
                ],
                &[
                    want_node(9),
                    want_node(10),
                    want_number(0),
                    want_name("body"),
                    want_file("item.ts"),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("an_empty_file_has_no_row"),
        word: Word::Succeeds,
        input: || "",
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[],
        }),
    },
];
fn subject(source: Input) -> Value {
    check::<PatternChildItem>();
    reading::<PatternChildItem>(source)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |source| Ok(subject(source)));
}
