//! The original coordinate assertions now belong to their validated value.
use super::value::CaseShard;
use ryo::cases::{Case, CaseName, Expect, Word, run};
type Input = (usize, usize);
type Value = (usize, usize);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Refusal {
    InvalidCoordinates,
}
const CASES: &[Case<Input, Value, Refusal>] = &[
    Case {
        name: CaseName("first zero-based partition"),
        word: Word::Succeeds,
        input: || (0, 3),
        expect: Expect::Value((0, 3)),
    },
    Case {
        name: CaseName("last bounded partition"),
        word: Word::Succeeds,
        input: || (2, 3),
        expect: Expect::Value((2, 3)),
    },
    Case {
        name: CaseName("index at count is refused"),
        word: Word::Refuses,
        input: || (3, 3),
        expect: Expect::Error(Refusal::InvalidCoordinates),
    },
    Case {
        name: CaseName("zero count is refused"),
        word: Word::Refuses,
        input: || (0, 0),
        expect: Expect::Error(Refusal::InvalidCoordinates),
    },
];
fn subject((index, count): Input) -> Result<Value, Refusal> {
    CaseShard::new(index, count)
        .map(|shard| (shard.index(), shard.count()))
        .map_err(|error| {
            assert_eq!(error.to_string(), "invalid case shard coordinates");
            Refusal::InvalidCoordinates
        })
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
