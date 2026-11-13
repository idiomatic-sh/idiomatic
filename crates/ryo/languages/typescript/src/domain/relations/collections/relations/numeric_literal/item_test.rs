//! Cases for TypeScript `numeric_literal` rows, checked against the shared reader contract: a literal's value, never its spelling.
use super::super::fixtures::contract::check;
use super::item::NumericLiteralItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::reading;
use ryo::languages::contract::{Reading, want_file, want_name, want_node, want_span};
type Input = &'static str;
type Value = Reading;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_spelling_of_one_is_the_float_one"),
        word: Word::Succeeds,
        input: || "1;0x1;1.0;1_0e-1;",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(2),
                    want_name("float"),
                    want_name("1.0"),
                    want_file("item.ts"),
                    want_span((1, 1), (1, 2)),
                ],
                &[
                    want_node(6),
                    want_name("float"),
                    want_name("1.0"),
                    want_file("item.ts"),
                    want_span((1, 3), (1, 6)),
                ],
                &[
                    want_node(10),
                    want_name("float"),
                    want_name("1.0"),
                    want_file("item.ts"),
                    want_span((1, 7), (1, 10)),
                ],
                &[
                    want_node(14),
                    want_name("float"),
                    want_name("1.0"),
                    want_file("item.ts"),
                    want_span((1, 11), (1, 17)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_bigint_is_an_integer_in_base_ten"),
        word: Word::Succeeds,
        input: || "0n;0x10n;",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(2),
                    want_name("integer"),
                    want_name("0"),
                    want_file("item.ts"),
                    want_span((1, 1), (1, 3)),
                ],
                &[
                    want_node(6),
                    want_name("integer"),
                    want_name("16"),
                    want_file("item.ts"),
                    want_span((1, 4), (1, 9)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_sign_is_not_part_of_the_literal"),
        word: Word::Succeeds,
        input: || "-0;",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(4),
                want_name("float"),
                want_name("0.0"),
                want_file("item.ts"),
                want_span((1, 2), (1, 3)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_large_float_is_written_without_an_exponent"),
        word: Word::Succeeds,
        input: || "1e21;",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(2),
                want_name("float"),
                want_name("1000000000000000000000.0"),
                want_file("item.ts"),
                want_span((1, 1), (1, 5)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_string_is_no_number"),
        word: Word::Succeeds,
        input: || "\"1\";",
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[],
        }),
    },
];
fn subject(source: Input) -> Value {
    check::<NumericLiteralItem>();
    reading::<NumericLiteralItem>(source)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |source| Ok(subject(source)));
}
