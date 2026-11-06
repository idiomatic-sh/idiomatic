//! The accepted and refused project scopes.
use crate::cases::{Case, CaseName, Expect, Word, run};
type Input = Option<Vec<String>>;
type Value = Option<usize>;
type Failure = &'static str;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an undeclared scope stays undeclared"),
        word: Word::Succeeds,
        input: || None,
        expect: Expect::Value(None),
    },
    Case {
        name: CaseName("a declared empty scope stays declared"),
        word: Word::Succeeds,
        input: || Some(Vec::new()),
        expect: Expect::Value(Some(0)),
    },
    Case {
        name: CaseName("a root that leaves the project is refused"),
        word: Word::Refuses,
        input: || Some(vec!["../outside".to_owned()]),
        expect: Expect::Error("unsafe root"),
    },
];
/// Reads one declared scope and counts its roots.
fn subject(roots: Input) -> Result<Value, Failure> {
    super::project_search_roots(roots)
        .map(|roots| roots.map(|roots| roots.len()))
        .map_err(|_refusal| "unsafe root")
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
