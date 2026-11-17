//! Cases for TypeScript `flow_block` rows, checked against the shared reader contract.
use super::super::fixtures::contract::check;
use super::item::FlowBlockItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::reading;
use ryo::languages::contract::{Reading, want_file, want_flag, want_number};
type Input = &'static str;
type Value = Reading;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_function_has_its_own_entry_and_exit_blocks"),
    word: Word::Succeeds,
    input: || "function f() {}",
    expect: Expect::Value(Reading {
        rows: &[
            &[
                want_number(0),
                want_flag(true),
                want_flag(true),
                want_flag(false),
                want_file("item.ts"),
            ],
            &[
                want_number(1),
                want_flag(true),
                want_flag(true),
                want_flag(false),
                want_file("item.ts"),
            ],
            &[
                want_number(2),
                want_flag(true),
                want_flag(true),
                want_flag(false),
                want_file("item.ts"),
            ],
            &[
                want_number(3),
                want_flag(true),
                want_flag(true),
                want_flag(true),
                want_file("item.ts"),
            ],
            &[
                want_number(4),
                want_flag(true),
                want_flag(true),
                want_flag(false),
                want_file("item.ts"),
            ],
        ],
        causes: &[],
    }),
}];
fn subject(input: Input) -> Result<Value, Failure> {
    Ok(reading::<FlowBlockItem>(input))
}
#[test]
fn succeeds() {
    check::<FlowBlockItem>();
    run(CASES, Word::Succeeds, subject);
}
