//! Cases for TypeScript parameter rows, checked against the shared reader contract.
use super::super::fixtures::contract::check;
use super::item::ParameterItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::reading;
use ryo::languages::contract::{Reading, want_file, want_name, want_node, want_number, want_span};
type Input = &'static str;
type Value = Reading;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_this_parameter_is_the_receiver"),
        word: Word::Succeeds,
        input: || "function f(this: Window) {}",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(7),
                want_name("receiver"),
                want_name("identifier"),
                want_name("this"),
                want_number(0),
                want_node(1),
                want_file("item.ts"),
                want_span((1, 12), (1, 24)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("positional_and_rest_parameters"),
        word: Word::Succeeds,
        input: || "function f(x: string, ...rest: number[]) {}",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(7),
                    want_name("positional"),
                    want_name("identifier"),
                    want_name("x"),
                    want_number(0),
                    want_node(1),
                    want_file("item.ts"),
                    want_span((1, 12), (1, 21)),
                ],
                &[
                    want_node(16),
                    want_name("variadic-positional"),
                    want_name("rest-element"),
                    want_name("rest"),
                    want_number(1),
                    want_node(1),
                    want_file("item.ts"),
                    want_span((1, 23), (1, 40)),
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
    check::<ParameterItem>();
    reading::<ParameterItem>(source)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |source| Ok(subject(source)));
}
