//! The `binding_read` item's cases, every row in full, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::BindingReadItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, reading};
use ryo::languages::contract::{Reading, want_file, want_name, want_node, want_number, want_span};

/// The source the item reads.
type Input = &'static str;
/// What the item writes for it: every row, every column shown, and why it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_unshadowed_builtin_is_builtin_and_an_unknown_global_is_unbound"),
        word: Word::Succeeds,
        input: || "Math();\nwindow();\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(3),
                    want_name("Math"),
                    want_node(0),
                    want_node(0),
                    want_name("value"),
                    want_name("builtin"),
                    want_number(0),
                    want_number(0),
                    want_file(SHOWN),
                    want_span((1, 1), (1, 5)),
                ],
                &[
                    want_node(11),
                    want_name("window"),
                    want_node(0),
                    want_node(0),
                    want_name("value"),
                    want_name("unbound"),
                    want_number(0),
                    want_number(0),
                    want_file(SHOWN),
                    want_span((2, 1), (2, 7)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_closure_read_resolves_outward_to_one_declaration"),
        word: Word::Succeeds,
        input: || "let x = 1;\nfunction f() { return x; }\n",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(28),
                want_name("x"),
                want_node(22),
                want_node(14),
                want_name("value"),
                want_name("unique-declaration"),
                want_number(0),
                want_number(0),
                want_file(SHOWN),
                want_span((2, 23), (2, 24)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_name_declared_twice_has_alternative_declarations"),
        word: Word::Succeeds,
        input: || "var x;\nvar x;\nx;\n",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(18),
                want_name("x"),
                want_node(0),
                want_node(0),
                want_name("value"),
                want_name("alternative-declarations"),
                want_number(0),
                want_number(0),
                want_file(SHOWN),
                want_span((3, 1), (3, 2)),
            ]],
            causes: &[],
        }),
    },
];

fn subject(source: Input) -> Value {
    check::<BindingReadItem>();
    reading::<BindingReadItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
