//! The `binding_use` item's cases, every row in full, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::BindingUseItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, reading};
use ryo::languages::contract::{Reading, want_file, want_name, want_node, want_span};

/// The source the item reads.
type Input = &'static str;
/// What the item writes for it: every row, every column shown, and why it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_write_to_a_constant_is_its_declarator"),
        word: Word::Succeeds,
        input: || "const x = 1;\nx = 2;\n",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(16),
                want_node(4),
                want_node(0),
                want_node(0),
                want_name("write"),
                want_name("unique"),
                want_file(SHOWN),
                want_span((2, 1), (2, 2)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_parameter_shadows_a_builtin"),
        word: Word::Succeeds,
        input: || "function f(Math) { Math(); }\n",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(16),
                want_node(7),
                want_node(11),
                want_node(1),
                want_name("value"),
                want_name("unique"),
                want_file(SHOWN),
                want_span((1, 20), (1, 24)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("an_increment_reads_and_writes"),
        word: Word::Succeeds,
        input: || "let x = 0;\nx++;\n",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(16),
                want_node(4),
                want_node(0),
                want_node(0),
                want_name("read-write"),
                want_name("unique"),
                want_file(SHOWN),
                want_span((2, 1), (2, 2)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_name_declared_twice_gives_one_row_for_each_declaration"),
        word: Word::Succeeds,
        input: || "var x;\nvar x;\nx;\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(18),
                    want_node(4),
                    want_node(0),
                    want_node(0),
                    want_name("value"),
                    want_name("alternative"),
                    want_file(SHOWN),
                    want_span((3, 1), (3, 2)),
                ],
                &[
                    want_node(18),
                    want_node(12),
                    want_node(0),
                    want_node(0),
                    want_name("value"),
                    want_name("alternative"),
                    want_file(SHOWN),
                    want_span((3, 1), (3, 2)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_type_reference_resolves_only_to_a_type"),
        word: Word::Succeeds,
        input: || "let T = 1;\ntype U = T;\n",
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[],
        }),
    },
];

fn subject(source: Input) -> Value {
    check::<BindingUseItem>();
    reading::<BindingUseItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
