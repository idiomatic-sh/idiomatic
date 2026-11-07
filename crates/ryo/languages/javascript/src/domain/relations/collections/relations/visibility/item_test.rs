//! Cases for JavaScript visibility rows, checked against the shared reader contract.
use super::super::fixtures::contract::check;
use super::item::VisibilityItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::reading;
use ryo::languages::contract::{Reading, want_file, want_name, want_node, want_span};
type Input = &'static str;
type Value = Reading;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_export_is_written_on_its_declaration"),
        word: Word::Succeeds,
        input: || "export function run() {}\nexport default class Shape {}\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(2),
                    want_node(4),
                    want_name("export"),
                    want_file("item.js"),
                    want_span((1, 1), (1, 7)),
                ],
                &[
                    want_node(17),
                    want_node(21),
                    want_name("export default"),
                    want_file("item.js"),
                    want_span((2, 1), (2, 15)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("an_exported_variable_declaration_is_written_on_each_declarator"),
        word: Word::Succeeds,
        input: || "export const a = 1, b = 2;\n",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(2),
                    want_node(7),
                    want_name("export"),
                    want_file("item.js"),
                    want_span((1, 1), (1, 7)),
                ],
                &[
                    want_node(2),
                    want_node(17),
                    want_name("export"),
                    want_file("item.js"),
                    want_span((1, 1), (1, 7)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("an_export_of_names_declares_nothing_here"),
        word: Word::Succeeds,
        input: || "const a = 1;\nexport { a };\n",
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[],
        }),
    },
];
fn subject(source: Input) -> Value {
    check::<VisibilityItem>();
    reading::<VisibilityItem>(source)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
