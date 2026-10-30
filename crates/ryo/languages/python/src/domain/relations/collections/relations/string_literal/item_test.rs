//! The `string_literal` item's cases, every row in full, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::StringLiteralItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, reading};
use ryo::languages::contract::{
    Reading, want_file, want_flag, want_name, want_node, want_span, want_unknown,
};

/// The source the item reads.
type Input = &'static str;
/// What the item writes for it: every row, every column shown, and why it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("strings_are_decoded_and_concatenations_joined"),
        word: Word::Succeeds,
        input: || "x = \"a\\n\" 'b'\nb\"bytes\"\n",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(7),
                want_node(0),
                want_name("a\nb"),
                want_file(SHOWN),
                want_span((1, 5), (1, 14)),
                want_flag(false),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("an_f_string_with_an_interpolation_is_unknown"),
        word: Word::Succeeds,
        input: || "f\"a{b}\"\nf\"static {{x}}\"\nt\"t{v}\"\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(2),
                    want_node(0),
                    want_unknown("interpolated-string"),
                    want_file(SHOWN),
                    want_span((1, 1), (1, 8)),
                    want_flag(false),
                ],
                &[
                    want_node(15),
                    want_node(0),
                    want_name("static {x}"),
                    want_file(SHOWN),
                    want_span((2, 1), (2, 16)),
                    want_flag(false),
                ],
                &[
                    want_node(23),
                    want_node(0),
                    want_unknown("interpolated-string"),
                    want_file(SHOWN),
                    want_span((3, 1), (3, 8)),
                    want_flag(false),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_format_spec_is_part_of_its_string_and_an_interpolated_f_string_is_one"),
        word: Word::Succeeds,
        input: || "f\"{x:>10}\"\nf\"{f'{y}'}\"\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(2),
                    want_node(0),
                    want_unknown("interpolated-string"),
                    want_file(SHOWN),
                    want_span((1, 1), (1, 11)),
                    want_flag(false),
                ],
                &[
                    want_node(17),
                    want_node(0),
                    want_unknown("interpolated-string"),
                    want_file(SHOWN),
                    want_span((2, 1), (2, 12)),
                    want_flag(false),
                ],
                &[
                    want_node(22),
                    want_node(0),
                    want_unknown("interpolated-string"),
                    want_file(SHOWN),
                    want_span((2, 4), (2, 10)),
                    want_flag(false),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("an_import_specifier_is_marked"),
        word: Word::Succeeds,
        input: || "m = importlib.import_module(\"pkg.mod\")",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(16),
                want_node(0),
                want_name("pkg.mod"),
                want_file(SHOWN),
                want_span((1, 29), (1, 38)),
                want_flag(true),
            ]],
            causes: &[],
        }),
    },
];

fn subject(source: Input) -> Value {
    check::<StringLiteralItem>();
    reading::<StringLiteralItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
