//! Cases for TypeScript `numeric_literal` rows, checked against the shared reader contract: a literal's value, never its spelling.
use super::super::fixtures::contract::check;
use super::item::NumericLiteralItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::reading;
use ryo::languages::contract::{Reading, want_file, want_flag, want_name, want_node, want_span};
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
                    want_flag(true),
                    want_file("item.ts"),
                    want_span((1, 1), (1, 2)),
                ],
                &[
                    want_node(6),
                    want_name("float"),
                    want_name("1.0"),
                    want_flag(true),
                    want_file("item.ts"),
                    want_span((1, 3), (1, 6)),
                ],
                &[
                    want_node(10),
                    want_name("float"),
                    want_name("1.0"),
                    want_flag(true),
                    want_file("item.ts"),
                    want_span((1, 7), (1, 10)),
                ],
                &[
                    want_node(14),
                    want_name("float"),
                    want_name("1.0"),
                    want_flag(true),
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
                    want_flag(true),
                    want_file("item.ts"),
                    want_span((1, 1), (1, 3)),
                ],
                &[
                    want_node(6),
                    want_name("integer"),
                    want_name("16"),
                    want_flag(true),
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
                want_flag(true),
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
                want_flag(true),
                want_file("item.ts"),
                want_span((1, 1), (1, 5)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("digits_a_number_rounds_are_not_exact_and_a_bigint_always_is"),
        word: Word::Succeeds,
        input: || "9007199254740993;0x20000000000001;0X1F_FFFF;0400000000000000001;9.0_07e15;9007199254740993n;",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(2),
                    want_name("float"),
                    want_name("9007199254740992.0"),
                    want_flag(false),
                    want_file("item.ts"),
                    want_span((1, 1), (1, 17)),
                ],
                &[
                    want_node(6),
                    want_name("float"),
                    want_name("9007199254740992.0"),
                    want_flag(false),
                    want_file("item.ts"),
                    want_span((1, 18), (1, 34)),
                ],
                &[
                    want_node(10),
                    want_name("float"),
                    want_name("2097151.0"),
                    want_flag(true),
                    want_file("item.ts"),
                    want_span((1, 35), (1, 44)),
                ],
                &[
                    want_node(14),
                    want_name("float"),
                    want_name("9007199254740992.0"),
                    want_flag(false),
                    want_file("item.ts"),
                    want_span((1, 45), (1, 64)),
                ],
                &[
                    want_node(18),
                    want_name("float"),
                    want_name("9007000000000000.0"),
                    want_flag(true),
                    want_file("item.ts"),
                    want_span((1, 65), (1, 74)),
                ],
                &[
                    want_node(22),
                    want_name("integer"),
                    want_name("9007199254740993"),
                    want_flag(true),
                    want_file("item.ts"),
                    want_span((1, 75), (1, 92)),
                ],
            ],
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
