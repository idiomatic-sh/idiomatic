//! Regex algorithm controls for UTF-8 spans and compilation refusal.
use super::service;
use super::types::{AuthoredRegex, CompilerReason, Haystack};
use crate::cases::{Case, CaseName, Expect, Word, run};

type Input = (
    &'static str,
    &'static str,
    &'static [std::ops::Range<usize>],
);
type Value = ();
type Failure = CompilerReason;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("utf8_offsets"),
        word: Word::Succeeds,
        input: || ("é", "éé", &[0..2, 2..4]),
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("nonoverlapping_source_order"),
        word: Word::Succeeds,
        input: || ("aa", "aaa", &[0..2]),
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("invalid_regex"),
        word: Word::Refuses,
        input: || ("[", "", &[]),
        expect: Expect::Refusal(|error| {
            crate::selects!(error.0.contains('[') && error.0.contains("unclosed character class"))
        }),
    },
];

fn subject((pattern, text, expected): Input) -> Result<Value, Failure> {
    let regex = service::compile_regex(AuthoredRegex(pattern))?;
    let mut ranges = Vec::new();
    let result = service::visit_regex_matches(&regex, Haystack(text), &mut |span| {
        ranges.push(span.0);
        std::ops::ControlFlow::Continue(())
    });
    assert_eq!(result, std::ops::ControlFlow::Continue(()));
    assert_eq!(ranges, expected);
    Ok(())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
