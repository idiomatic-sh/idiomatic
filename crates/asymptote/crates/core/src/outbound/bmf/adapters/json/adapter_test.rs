//! The serde-json adapter of the bmf port: the contract, then a tool's real output and the texts it refuses.

use super::adapter::BmfJsonAdapter;
use crate::cases::{Case, CaseName, Expect, Selection, Word, run};
use crate::outbound::bmf::errors::BmfError;
use crate::outbound::bmf::fixtures::contract::check;
use crate::outbound::bmf::interface::BmfPort as _;
use crate::outbound::bmf::types::BmfText;

/// What the ryo bench prints: two benchmarks, three measures, a spread on one.
const RYO: &str = r#"{"check-ryo": {"wall-ms": {"value": 831.4, "lower_value": 820.0, "upper_value": 850.2}, "cpu-ms": {"value": 2400}, "peak-mb": {"value": 150}}, "up-ryo": {"wall-ms": {"value": 45}}}"#;

/// What is read: the contract, or a text.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// The port's contract.
    Contract,
    /// A text.
    Text(&'static str),
}

type Input = Question;
type Value = usize;
type Failure = BmfError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_adapter_keeps_the_contract"),
        word: Word::Succeeds,
        input: || Question::Contract,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("a_tools_output_is_read_measure_by_measure"),
        word: Word::Succeeds,
        input: || Question::Text(RYO),
        expect: Expect::Value(4),
    },
    Case {
        name: CaseName("output_that_is_not_json_is_refused"),
        word: Word::Refuses,
        input: || Question::Text("not json"),
        expect: Expect::Refusal(|actual| {
            if matches!(actual, BmfError::Unparsable { .. }) {
                Selection::Matches
            } else {
                Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_number_where_a_benchmark_is_expected_is_refused"),
        word: Word::Refuses,
        input: || Question::Text(r#"{"b": 1}"#),
        expect: Expect::Refusal(|actual| {
            if matches!(actual, BmfError::Unparsable { .. }) {
                Selection::Matches
            } else {
                Selection::Differs
            }
        }),
    },
];

fn subject(question: Input) -> Result<Value, Failure> {
    match question {
        Question::Contract => {
            check(&BmfJsonAdapter);
            Ok(0)
        }
        Question::Text(text) => BmfJsonAdapter
            .read(&BmfText(text.into()))
            .map(|read| read.0.len()),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
