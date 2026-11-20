//! The `follow` call, from the compile inbound API: the step it compiles to, where it runs, and what it refuses.

use super::super::compiled_steps;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::query_operators::types::OperationKind;
use std::borrow::Cow;

/// The query text a row gives.
type Input = &'static str;
/// The plan's steps of the kind this part compiles, as the plan holds them, each starred when it runs after the files.
type Value = Cow<'static, str>;
/// What the compile use case refuses a row with, as its caller reads it.
type Failure = Cow<'static, str>;

/// Pointers followed within each scope, name and file.
const KEYED: &str = r#"let pointers = select scope, name, file, ordinal as node, ordinal as target from "binding-event"; let a = follow(edges=pointers, source="node", destination="target", keys=[scope, name, file]); emit a subject file key(scope, name, node, target);"#;
/// No keys: one graph of every file's pointers, so the step runs after the files.
const UNKEYED: &str = r#"let pointers = select scope, name, file, ordinal as node, ordinal as target from "binding-event"; let a = follow(edges=pointers, source="node", destination="target"); emit a subject node key(target);"#;
/// A source and a destination of two kinds.
const MIXED: &str = r#"let pointers = select scope, name, file, ordinal as node, ordinal as target from "binding-event"; let a = follow(edges=pointers, source="scope", destination="target", keys=[file]); emit a subject file key(target);"#;
/// A key that is the source.
const KEY_IS_AN_END: &str = r#"let pointers = select scope, name, file, ordinal as node, ordinal as target from "binding-event"; let a = follow(edges=pointers, source="node", destination="target", keys=[node]); emit a subject node key(target);"#;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("follow_compiles_to_one_step_keys_first"),
        word: Word::Succeeds,
        input: || KEYED,
        expect: Expect::Value(Cow::Borrowed(
            "Follow { edges: Step(StepId(0)), keys: [ColumnIndex(0), ColumnIndex(1), ColumnIndex(2)], source: ColumnIndex(3), destination: ColumnIndex(4) }",
        )),
    },
    Case {
        name: CaseName("follow_without_keys_is_one_graph_after_the_files"),
        word: Word::Succeeds,
        input: || UNKEYED,
        expect: Expect::Value(Cow::Borrowed(
            "Follow { edges: Step(StepId(0)), keys: [], source: ColumnIndex(3), destination: ColumnIndex(4) }*",
        )),
    },
    Case {
        name: CaseName("a_source_and_a_destination_of_two_kinds_are_refused"),
        word: Word::Refuses,
        input: || MIXED,
        expect: Expect::Error(Cow::Borrowed(
            "a closure, follow or cycles step needs a source and a destination of one kind (from `<query>` at byte 98)",
        )),
    },
    Case {
        name: CaseName("a_key_that_is_an_end_of_the_edge_is_refused"),
        word: Word::Refuses,
        input: || KEY_IS_AN_END,
        expect: Expect::Error(Cow::Borrowed(
            "output column `node` already exists (from `<query>` at byte 98)",
        )),
    },
];

fn subject(text: Input) -> Result<Value, Failure> {
    compiled_steps(text, &[OperationKind::Follow])
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
