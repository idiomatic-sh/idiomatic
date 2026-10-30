//! The `annotates` item's cases, every row as its cells, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::AnnotatesItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, reading};
use ryo::languages::contract::{Reading, want_file, want_name, want_node, want_span};

/// The source the item reads.
type Input = &'static str;
/// What the item writes for it: every cell of every row, and why it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_named_parameter_and_result_is_annotated_with_its_type_as_written"),
        word: Word::Succeeds,
        input: || "package p\nfunc f(a int, b ...string) (n int, error) {}\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(18),
                    want_node(15),
                    want_name("int"),
                    want_file(SHOWN),
                    want_span((2, 10), (2, 13)),
                ],
                &[
                    want_node(26),
                    want_node(23),
                    want_name("...string"),
                    want_file(SHOWN),
                    want_span((2, 17), (2, 26)),
                ],
                &[
                    want_node(38),
                    want_node(35),
                    want_name("int"),
                    want_file(SHOWN),
                    want_span((2, 31), (2, 34)),
                ],
                &[
                    want_node(43),
                    want_node(42),
                    want_name("error"),
                    want_file(SHOWN),
                    want_span((2, 36), (2, 41)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_field_is_annotated_and_an_embedded_one_by_its_own_type"),
        word: Word::Succeeds,
        input: || "package p\ntype T struct {\n\tx, y int\n\t*Base\n}\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(27),
                    want_node(20),
                    want_name("int"),
                    want_file(SHOWN),
                    want_span((3, 7), (3, 10)),
                ],
                &[
                    want_node(27),
                    want_node(24),
                    want_name("int"),
                    want_file(SHOWN),
                    want_span((3, 7), (3, 10)),
                ],
                &[
                    want_node(31),
                    want_node(30),
                    want_name("*Base"),
                    want_file(SHOWN),
                    want_span((4, 2), (4, 7)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName(
            "a_declaration_with_a_type_annotates_each_name_and_one_without_annotates_none",
        ),
        word: Word::Succeeds,
        input: || "package p\nvar a, b int = 1, 2\nvar c = 3\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(17),
                    want_node(10),
                    want_name("int"),
                    want_file(SHOWN),
                    want_span((2, 10), (2, 13)),
                ],
                &[
                    want_node(17),
                    want_node(14),
                    want_name("int"),
                    want_file(SHOWN),
                    want_span((2, 10), (2, 13)),
                ],
            ],
            causes: &[],
        }),
    },
];

fn subject(source: Input) -> Value {
    check::<AnnotatesItem>();
    reading::<AnnotatesItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
