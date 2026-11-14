//! Cases for TypeScript `flow_member` rows, checked against the shared reader contract.
use super::super::fixtures::contract::check;
use super::item::FlowMemberItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::reading;
use ryo::languages::contract::{Reading, want_file, want_node, want_number};
type Input = &'static str;
type Value = Reading;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_function_and_its_parts_are_in_its_entry_block"),
    word: Word::Succeeds,
    input: || "function f() {}",
    expect: Expect::Value(Reading {
        rows: &[
            &[want_node(0), want_number(1), want_file("item.ts")],
            &[want_node(1), want_number(3), want_file("item.ts")],
            &[want_node(4), want_number(3), want_file("item.ts")],
            &[want_node(9), want_number(3), want_file("item.ts")],
        ],
        causes: &[],
    }),
}];
fn subject(input: Input) -> Result<Value, Failure> {
    Ok(reading::<FlowMemberItem>(input))
}
#[test]
fn succeeds() {
    check::<FlowMemberItem>();
    run(CASES, Word::Succeeds, subject);
}
