//! The `string_literal` item's cases, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::StringLiteralItem;
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
        name: CaseName("a_constant_is_a_literal"),
        word: Word::Succeeds,
        input: || "const VALUE: &str = \"value\";",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(18),
                want_node(0),
                want_name("value"),
                want_file(SHOWN),
                want_span((1, 21), (1, 28)),
                want_flag(false),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("an_escape_is_read"),
        word: Word::Succeeds,
        input: || "fn run() { let text = \"a\\tb\"; }",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(23),
                want_node(1),
                want_name("a\tb"),
                want_file(SHOWN),
                want_span((1, 23), (1, 29)),
                want_flag(false),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_literal_in_the_arguments_of_a_standard_macro_is_a_literal"),
        word: Word::Succeeds,
        input: || "fn f(x: &str) -> bool { matches!(x, \"__replace__\") }",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(58),
                want_node(1),
                want_name("__replace__"),
                want_file(SHOWN),
                want_span((1, 37), (1, 50)),
                want_flag(false),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("an_import_specifier_is_marked"),
        word: Word::Succeeds,
        input: || "#[path = \"other.rs\"]\nmod other;",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(13),
                want_node(0),
                want_name("other.rs"),
                want_file(SHOWN),
                want_span((1, 10), (1, 20)),
                want_flag(true),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_string_in_an_attribute_token_tree_is_a_literal"),
        word: Word::Succeeds,
        input: || "#[rustfmt::skip(v = r\"x\", w = \"\\x41\")]\nfn f() {}",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(21),
                    want_node(1),
                    want_name("x"),
                    want_file(SHOWN),
                    want_span((1, 21), (1, 25)),
                    want_flag(false),
                ],
                &[
                    want_node(28),
                    want_node(1),
                    want_name("A"),
                    want_file(SHOWN),
                    want_span((1, 31), (1, 37)),
                    want_flag(false),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_string_in_the_token_tree_of_another_macro_is_not_read"),
        word: Word::Succeeds,
        input: || "fn f() { other!(\"x\"); }",
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_string_in_a_cfg_predicate_or_an_abi_is_a_literal"),
        word: Word::Succeeds,
        input: || "#[cfg(feature = \"a\")]\nextern r\"C\" fn f() {}",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(13),
                    want_node(1),
                    want_name("a"),
                    want_file(SHOWN),
                    want_span((1, 17), (1, 20)),
                    want_flag(false),
                ],
                &[
                    want_node(20),
                    want_node(1),
                    want_name("C"),
                    want_file(SHOWN),
                    want_span((2, 8), (2, 12)),
                    want_flag(false),
                ],
            ],
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
