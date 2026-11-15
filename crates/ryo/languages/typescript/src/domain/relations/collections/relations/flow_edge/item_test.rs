//! Cases for TypeScript `flow_edge` rows, checked against the shared reader contract.
use super::super::fixtures::contract::check;
use super::item::FlowEdgeItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::reading;
use ryo::languages::contract::{Reading, want_file, want_name, want_number};
type Input = &'static str;
type Value = Reading;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_function_is_entered_by_a_new_function_edge"),
    word: Word::Succeeds,
    input: || "function f() {}",
    expect: Expect::Value(Reading {
        rows: &[
            &[
                want_number(1),
                want_number(0),
                want_name("error-implicit"),
                want_file("item.ts"),
            ],
            &[
                want_number(1),
                want_number(3),
                want_name("new-function"),
                want_file("item.ts"),
            ],
            &[
                want_number(1),
                want_number(4),
                want_name("normal"),
                want_file("item.ts"),
            ],
            &[
                want_number(3),
                want_number(2),
                want_name("error-implicit"),
                want_file("item.ts"),
            ],
            &[
                want_number(4),
                want_number(0),
                want_name("error-implicit"),
                want_file("item.ts"),
            ],
        ],
        causes: &[],
    }),
}];
fn subject(input: Input) -> Result<Value, Failure> {
    Ok(reading::<FlowEdgeItem>(input))
}
#[test]
fn succeeds() {
    check::<FlowEdgeItem>();
    run(CASES, Word::Succeeds, subject);
}
