//! What the Bencher Metric Format schemas accept and refuse.

use super::read;
use crate::cases::{Case, CaseName, Expect, Selection, Word, run};

/// The text.
type Input = &'static str;
/// The measurements read, `benchmark/measure=value[lower,upper]`, joined.
type Value = std::borrow::Cow<'static, str>;
type Failure = String;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_metric_with_both_bounds_is_read"),
        word: Word::Succeeds,
        input: || r#"{"b": {"latency": {"value": 3.5, "lower_value": 3, "upper_value": 4}}}"#,
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "b/latency=3.5[Some(3.0),Some(4.0)]",
        )),
    },
    Case {
        name: CaseName("a_metric_with_a_value_alone_is_read"),
        word: Word::Succeeds,
        input: || r#"{"b": {"wall-ms": {"value": 831}}}"#,
        expect: Expect::Value(std::borrow::Cow::Borrowed("b/wall-ms=831[None,None]")),
    },
    Case {
        name: CaseName("measures_are_sorted_by_benchmark_then_measure"),
        word: Word::Succeeds,
        input: || r#"{"b": {"y": {"value": 2}, "x": {"value": 1}}, "a": {"z": {"value": 3}}}"#,
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "a/z=3[None,None] b/x=1[None,None] b/y=2[None,None]",
        )),
    },
    Case {
        name: CaseName("a_spread_whose_high_end_is_under_its_value_is_refused"),
        word: Word::Refuses,
        input: || r#"{"b": {"m": {"value": 1000, "lower_value": 1, "upper_value": 2}}}"#,
        expect: Expect::Refusal(|actual| {
            selects(actual, "b/m: lower_value <= value <= upper_value")
        }),
    },
    Case {
        name: CaseName("a_spread_whose_low_end_is_over_its_value_is_refused"),
        word: Word::Refuses,
        input: || r#"{"b": {"m": {"value": 1, "lower_value": 2}}}"#,
        expect: Expect::Refusal(|actual| {
            selects(actual, "b/m: lower_value <= value <= upper_value")
        }),
    },
    Case {
        name: CaseName("an_unnamed_benchmark_is_refused"),
        word: Word::Refuses,
        input: || r#"{"": {"wall-ms": {"value": 1}}}"#,
        expect: Expect::Refusal(|actual| selects(actual, "are named")),
    },
    Case {
        name: CaseName("an_unnamed_measure_is_refused"),
        word: Word::Refuses,
        input: || r#"{"b": {"": {"value": 1}}}"#,
        expect: Expect::Refusal(|actual| selects(actual, "are named")),
    },
    Case {
        name: CaseName("an_array_is_refused"),
        word: Word::Refuses,
        input: || "[]",
        expect: Expect::Refusal(|actual| selects(actual, "invalid type: sequence")),
    },
    Case {
        name: CaseName("a_metric_without_a_value_is_refused"),
        word: Word::Refuses,
        input: || r#"{"b": {"m": {"lower_value": 1}}}"#,
        expect: Expect::Refusal(|actual| selects(actual, "missing field `value`")),
    },
    Case {
        name: CaseName("a_non_numeric_value_is_refused"),
        word: Word::Refuses,
        input: || r#"{"b": {"m": {"value": "fast"}}}"#,
        expect: Expect::Refusal(|actual| selects(actual, "invalid type: string")),
    },
    Case {
        name: CaseName("a_non_numeric_bound_is_refused"),
        word: Word::Refuses,
        input: || r#"{"b": {"m": {"value": 1, "upper_value": "x"}}}"#,
        expect: Expect::Refusal(|actual| selects(actual, "invalid type: string")),
    },
    Case {
        name: CaseName("a_metric_that_is_a_number_is_refused"),
        word: Word::Refuses,
        input: || r#"{"b": {"m": 1}}"#,
        expect: Expect::Refusal(|actual| selects(actual, "invalid type")),
    },
    Case {
        name: CaseName("a_key_bencher_does_not_define_is_refused"),
        word: Word::Refuses,
        input: || r#"{"b": {"m": {"value": 1, "median": 2}}}"#,
        expect: Expect::Refusal(|actual| selects(actual, "unknown field `median`")),
    },
    Case {
        name: CaseName("a_negative_value_is_refused_naming_the_benchmark_and_measure"),
        word: Word::Refuses,
        input: || r#"{"b": {"m": {"value": -1}}}"#,
        expect: Expect::Refusal(|actual| {
            selects(
                actual,
                "b/m: a value and its bounds are numbers of zero or more",
            )
        }),
    },
    Case {
        name: CaseName("a_negative_bound_is_refused"),
        word: Word::Refuses,
        input: || r#"{"b": {"m": {"value": 1, "lower_value": -0.5}}}"#,
        expect: Expect::Refusal(|actual| selects(actual, "b/m:")),
    },
    Case {
        name: CaseName("an_object_with_no_measure_is_refused"),
        word: Word::Refuses,
        input: || r#"{"b": {}}"#,
        expect: Expect::Refusal(|actual| selects(actual, "it holds no measurement")),
    },
];

fn selects(actual: &str, expected: &str) -> Selection {
    if actual.contains(expected) {
        Selection::Matches
    } else {
        Selection::Differs
    }
}

fn subject(text: Input) -> Result<Value, Failure> {
    read(text).map(|read| {
        std::borrow::Cow::Owned(
            read.0
                .iter()
                .map(|each| {
                    let bound = |spread: &crate::domain::perf::types::Spread| match spread {
                        crate::domain::perf::types::Spread::Given(number) => {
                            format!("Some({:?})", number.0)
                        }
                        crate::domain::perf::types::Spread::Absent => "None".to_owned(),
                    };
                    format!(
                        "{}/{}={}[{},{}]",
                        each.benchmark.0,
                        each.measure.0,
                        each.value.0,
                        bound(&each.lower),
                        bound(&each.upper)
                    )
                })
                .collect::<Vec<_>>()
                .join(" "),
        )
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
