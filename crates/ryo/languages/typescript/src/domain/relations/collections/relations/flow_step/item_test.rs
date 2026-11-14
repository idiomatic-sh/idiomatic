//! Cases for TypeScript `flow_step` rows, checked against the shared reader contract.
use super::super::fixtures::contract::check;
use super::item::FlowStepItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::reading;
use ryo::languages::contract::{Reading, want_file, want_name, want_node, want_number};
type Input = &'static str;
type Value = Reading;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_step_after_a_return_keeps_its_place_after_the_mark"),
    word: Word::Succeeds,
    input: || "function h() { return; foo(); }",
    expect: Expect::Value(Reading {
        rows: &[
            &[
                want_number(3),
                want_number(0),
                want_name("return-undefined"),
                want_node(12),
                want_file("item.ts"),
            ],
            &[
                want_number(4),
                want_number(1),
                want_name("statement"),
                want_node(16),
                want_file("item.ts"),
            ],
        ],
        causes: &[],
    }),
}];
fn subject(input: Input) -> Result<Value, Failure> {
    Ok(reading::<FlowStepItem>(input))
}
#[test]
fn succeeds() {
    check::<FlowStepItem>();
    run(CASES, Word::Succeeds, subject);
}
