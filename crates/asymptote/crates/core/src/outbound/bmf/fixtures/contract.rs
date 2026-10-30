//! What every `bmf` adapter must pass: a metric is read back with its median and spread, and no output, and a
//! benchmark or measure with no name, is no measurement.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::bmf::errors::BmfError;
use crate::outbound::bmf::interface::BmfPort;
use crate::outbound::bmf::types::BmfText;

/// Which question is asked.
#[derive(Clone, Copy, Debug)]
pub enum Question {
    /// A metric with its spread is read back.
    Metric,
    /// A metric with its median alone is read back.
    Median,
    /// No output is refused.
    Empty,
    /// An object that is no benchmark is refused.
    NotAnObject,
    /// A benchmark with no name is refused.
    UnnamedBenchmark,
    /// A measure with no name is refused.
    UnnamedMeasure,
}

/// The output a row reads, named by its question.
type Input = Question;
/// The measurements read back, `benchmark/measure=value`, joined.
type Value = std::borrow::Cow<'static, str>;
/// The refusal a row without measurements expects.
type Failure = BmfError;

/// The contract's rows: every implementation runs them.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_metric_reads_back_as_its_median"),
        word: Word::Succeeds,
        input: || Question::Metric,
        expect: Expect::Value(std::borrow::Cow::Borrowed("b/wall-ms=831")),
    },
    Case {
        name: CaseName("a_metric_without_a_spread_reads_back_as_its_median"),
        word: Word::Succeeds,
        input: || Question::Median,
        expect: Expect::Value(std::borrow::Cow::Borrowed("b/peak-mb=150")),
    },
    Case {
        name: CaseName("no_output_is_no_measurement"),
        word: Word::Refuses,
        input: || Question::Empty,
        expect: Expect::Refusal(|actual| {
            if matches!(actual, BmfError::Unparsable { .. }) {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_benchmark_with_no_name_is_no_measurement"),
        word: Word::Refuses,
        input: || Question::UnnamedBenchmark,
        expect: Expect::Refusal(|actual| {
            if matches!(actual, BmfError::Unparsable { .. }) {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_measure_with_no_name_is_no_measurement"),
        word: Word::Refuses,
        input: || Question::UnnamedMeasure,
        expect: Expect::Refusal(|actual| {
            if matches!(actual, BmfError::Unparsable { .. }) {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_list_is_no_measurement"),
        word: Word::Refuses,
        input: || Question::NotAnObject,
        expect: Expect::Refusal(|actual| {
            if matches!(actual, BmfError::Unparsable { .. }) {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
];

/// Runs the contract's rows against one implementation.
///
/// # Panics
///
/// When the implementation breaks the contract.
pub fn check(adapter: &impl BmfPort) {
    let subject = |question: Input| -> Result<Value, Failure> {
        let text = match question {
            Question::Metric => {
                r#"{"b": {"wall-ms": {"value": 831, "lower_value": 800, "upper_value": 860}}}"#
            }
            Question::Median => r#"{"b": {"peak-mb": {"value": 150}}}"#,
            Question::Empty => "",
            Question::NotAnObject => "[1, 2]",
            Question::UnnamedBenchmark => r#"{"": {"wall-ms": {"value": 1}}}"#,
            Question::UnnamedMeasure => r#"{"b": {"": {"value": 1}}}"#,
        };
        let read = adapter.read(&BmfText(text.into()))?;
        Ok(std::borrow::Cow::Owned(
            read.0
                .iter()
                .map(|each| format!("{}/{}={}", each.benchmark.0, each.measure.0, each.value.0))
                .collect::<Vec<_>>()
                .join(" "),
        ))
    };
    run(CASES, Word::Succeeds, subject);
    run(CASES, Word::Refuses, subject);
}
