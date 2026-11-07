//! Cases for TypeScript `syntax_child` rows, checked against the shared reader contract.
use super::super::fixtures::contract::check;
use super::item::SyntaxChildItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::reading;
use ryo::languages::contract::{Reading, want_file, want_name, want_node, want_number};
type Input = &'static str;
type Value = Reading;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("ordered_syntax_children"),
        word: Word::Succeeds,
        input: || "x;",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(0),
                    want_node(1),
                    want_number(0),
                    want_name("body"),
                    want_file("item.ts"),
                ],
                &[
                    want_node(1),
                    want_node(2),
                    want_number(0),
                    want_name("expression"),
                    want_file("item.ts"),
                ],
                &[
                    want_node(1),
                    want_node(4),
                    want_number(1),
                    want_name("none"),
                    want_file("item.ts"),
                ],
                &[
                    want_node(2),
                    want_node(3),
                    want_number(0),
                    want_name("none"),
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
    check::<SyntaxChildItem>();
    reading::<SyntaxChildItem>(source)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |source| Ok(subject(source)));
}
