//! The public lexical-origin row is typed and incomplete syntax stays explicitly partial.
use super::super::fixtures::contract::check;
use super::item::QualifiedNameItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, reading};
use ryo::languages::contract::{Reading, want_file, want_name, want_node, want_number, want_span};
type Input = &'static str;
type Value = Reading;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("builtin version does not assume the query target"),
        word: Word::Succeeds,
        input: || "ExceptionGroup\n",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(2),
                want_name("builtins.ExceptionGroup"),
                want_name("builtin"),
                want_number(3),
                want_number(11),
                want_file(SHOWN),
                want_span((1, 1), (1, 15)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("incomplete syntax does not pretend complete lexical provenance"),
        word: Word::Succeeds,
        input: || "if:\n    pass\n",
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &["qualified names require a complete syntax tree"],
        }),
    },
];
fn subject(source: Input) -> Value {
    check::<QualifiedNameItem>();
    reading::<QualifiedNameItem>(source)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}

#[test]
fn complete_ast_semantic_error_does_not_hide_qualified_names() {
    let result = subject("global x\nnonlocal x\nx\n");
    assert!(result.causes.is_empty());
    assert_eq!(result.rows.len(), 1);
}
