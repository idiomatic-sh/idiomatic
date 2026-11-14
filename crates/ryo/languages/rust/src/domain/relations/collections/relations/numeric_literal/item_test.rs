//! The `numeric_literal` item's cases, and the collection's shared cases: a literal's value, never its spelling.

use super::super::fixtures::contract::check;
use super::item::NumericLiteralItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, reading};
use ryo::languages::contract::{Reading, want_file, want_flag, want_name, want_node, want_span};

/// The source the item reads.
type Input = &'static str;
/// What the item writes for it: every cell of every row, and the causes for which it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_spelling_of_sixteen_is_the_integer_sixteen"),
        word: Word::Succeeds,
        input: || "const A: [u8; 3] = [16, 0x10, 0b1_0000];",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(18),
                    want_name("integer"),
                    want_name("3"),
                    want_flag(true),
                    want_file(SHOWN),
                    want_span((1, 15), (1, 16)),
                ],
                &[
                    want_node(26),
                    want_name("integer"),
                    want_name("16"),
                    want_flag(true),
                    want_file(SHOWN),
                    want_span((1, 21), (1, 23)),
                ],
                &[
                    want_node(30),
                    want_name("integer"),
                    want_name("16"),
                    want_flag(true),
                    want_file(SHOWN),
                    want_span((1, 25), (1, 29)),
                ],
                &[
                    want_node(34),
                    want_name("integer"),
                    want_name("16"),
                    want_flag(true),
                    want_file(SHOWN),
                    want_span((1, 31), (1, 39)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_float_is_its_binary64_value"),
        word: Word::Succeeds,
        input: || "const B: f64 = 1_0e-1;",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(16),
                want_name("float"),
                want_name("1.0"),
                want_flag(true),
                want_file(SHOWN),
                want_span((1, 16), (1, 22)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("an_integer_with_a_float_suffix_is_a_float"),
        word: Word::Succeeds,
        input: || "const C: f32 = 1f32;",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(16),
                want_name("float"),
                want_name("1.0"),
                want_flag(true),
                want_file(SHOWN),
                want_span((1, 16), (1, 20)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_sign_is_not_part_of_the_literal"),
        word: Word::Succeeds,
        input: || "const D: i8 = -1;",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(18),
                want_name("integer"),
                want_name("1"),
                want_flag(true),
                want_file(SHOWN),
                want_span((1, 16), (1, 17)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_literal_pattern_is_a_literal"),
        word: Word::Succeeds,
        input: || "fn e(x: u8) -> bool { matches!(x, 1..=9) }",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(61),
                    want_name("integer"),
                    want_name("1"),
                    want_flag(true),
                    want_file(SHOWN),
                    want_span((1, 35), (1, 36)),
                ],
                &[
                    want_node(65),
                    want_name("integer"),
                    want_name("9"),
                    want_flag(true),
                    want_file(SHOWN),
                    want_span((1, 39), (1, 40)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_tuple_field_is_no_literal"),
        word: Word::Succeeds,
        input: || "fn f(t: (u8,)) -> u8 { t.0 }",
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_string_is_no_number"),
        word: Word::Succeeds,
        input: || "const G: &str = \"1\";",
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_literal_the_compiler_refuses_is_read_in_part"),
        word: Word::Succeeds,
        input: || "const H: u8 = 0b102;",
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &["a numeric literal the compiler refuses has no value"],
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
