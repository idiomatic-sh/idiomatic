//! Regex algorithm controls for UTF-8 spans and compilation refusal.
use super::service;
use crate::cases::{Case, CaseName, Expect, Word, run};

type Input = (&'static str, &'static str);
type Value = Vec<std::ops::Range<usize>>;
type Failure = String;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case { name: CaseName("utf8_offsets"), word: Word::Succeeds,
        input: || ("é", "éé"), expect: Expect::Value(vec![0..2, 2..4]) },
    Case { name: CaseName("nonoverlapping_source_order"), word: Word::Succeeds,
        input: || ("aa", "aaa"), expect: Expect::Value(vec![0..2]) },
    Case { name: CaseName("invalid_regex"), word: Word::Refuses,
        input: || ("[", ""), expect: Expect::Refusal(|error| crate::cases::selects(!error.is_empty())) },
];

fn subject((pattern, text): Input) -> Result<Value, Failure> {
    let regex = service::compile_regex(pattern)?;
    let mut ranges = Vec::new();
    let result = service::visit_regex_matches(&regex, text, &mut |range| {
        ranges.push(range);
        std::ops::ControlFlow::Continue(())
    });
    assert_eq!(result, std::ops::ControlFlow::Continue(()));
    Ok(ranges)
}

#[test]
fn succeeds() { run(CASES, Word::Succeeds, subject); }
#[test]
fn refuses() { run(CASES, Word::Refuses, subject); }
