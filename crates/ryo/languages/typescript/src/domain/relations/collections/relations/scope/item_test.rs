//! Cases for TypeScript scope rows, checked against the shared reader contract.
use super::super::fixtures::contract::check;
use super::item::ScopeItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::reading;
use ryo::languages::contract::{Reading, want_file, want_node, want_span};
type Input = &'static str;
type Value = Reading;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("file_function_and_block_scopes"),
    word: Word::Succeeds,
    input: || "function f() {}",
    expect: Expect::Value(Reading {
        rows: &[
            &[
                want_node(0),
                want_node(0),
                want_file("item.ts"),
                want_span((1, 1), (1, 16)),
            ],
            &[
                want_node(0),
                want_node(1),
                want_file("item.ts"),
                want_span((1, 1), (1, 16)),
            ],
            &[
                want_node(1),
                want_node(9),
                want_file("item.ts"),
                want_span((1, 14), (1, 16)),
            ],
        ],
        causes: &[],
    }),
}];
fn subject(source: Input) -> Value {
    check::<ScopeItem>();
    reading::<ScopeItem>(source)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |source| Ok(subject(source)));
}
