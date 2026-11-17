//! What a run's measurements mean against the lock's perf records, and the verdict they give.

use super::service::{judge, resolved, rewritten};
use super::types::{Measured, PerfOutcome, Spread, ToleranceKey};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::judgement::service::with_perf;
use crate::domain::judgement::types::Verdict;
use crate::domain::manifest::types::Percent;
use crate::domain::operations::types::{
    BenchmarkName, MeasureName, Number, PerfKey, PerfRecord, Reason,
};
use crate::domain::violations::types::ToolId;
use std::collections::BTreeMap;

/// Which question is asked.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// A median over the best, with no spread.
    Over,
    /// A median exactly at the best.
    AtBest,
    /// A median under the best, with no spread.
    Under,
    /// A spread whose low end is over the best.
    SpreadAbove,
    /// A spread that holds the best.
    SpreadAround,
    /// A spread whose high end is under the best.
    SpreadBelow,
    /// A spread whose high end is under the best around a value over it: never a raise.
    SpreadBelowValueAbove,
    /// A median over the best by less than the tool's tolerance.
    WithinTolerance,
    /// A median over the best by exactly the tool's tolerance.
    AtTolerance,
    /// A median over the best by more than the tool's tolerance.
    BeyondTolerance,
    /// A median under the best, whatever the tolerance.
    UnderWithTolerance,
    /// A tolerance of another measure of the tool.
    ToleranceOfAnotherMeasure,
    /// A median with no record.
    Unseeded,
    /// A recorded metric the tool no longer reports.
    Dropped,
    /// A recorded metric of a tool that did not complete.
    ToolDown,
    /// What an improvement rewrites.
    Rewrite,
    /// The verdict of a pass, with a regression.
    VerdictRegressed,
    /// The verdict of a pass, with only an improvement.
    VerdictImproved,
    /// The verdict of a failed tool, with a regression.
    VerdictUndetermined,
    /// The records a tool no longer reports.
    Gone,
}

type Input = Question;
type Value = std::borrow::Cow<'static, str>;
type Failure = String;

fn key(tool: &str, name: &str) -> PerfKey {
    PerfKey {
        tool: ToolId(tool.into()),
        benchmark: BenchmarkName("check".into()),
        measure: MeasureName(name.into()),
    }
}

/// The record of `wall-ms` of tool `bench`: best 100.
fn record() -> PerfRecord {
    PerfRecord {
        key: key("bench", "wall-ms"),
        best: Number(100.0),
        reasons: vec![Reason("seeded".into())],
    }
}

fn measured(value: f64) -> Measured {
    spread(value, Spread::Absent, Spread::Absent)
}

fn spread(value: f64, lower: Spread, upper: Spread) -> Measured {
    Measured {
        key: PerfKey {
            tool: ToolId("bench".into()),
            benchmark: BenchmarkName("check".into()),
            measure: MeasureName("wall-ms".into()),
        },
        value: Number(value),
        lower,
        upper,
    }
}

fn between(lower: f64, upper: f64) -> Measured {
    spread(
        f64::midpoint(lower, upper),
        Spread::Given(Number(lower)),
        Spread::Given(Number(upper)),
    )
}

/// No tolerance: every tool is held to its best.
fn none() -> BTreeMap<ToleranceKey, Percent> {
    BTreeMap::new()
}

/// The measure `wall-ms` of the tool `bench` allowed `percent` over its best.
fn allowing(percent: u32) -> BTreeMap<ToleranceKey, Percent> {
    BTreeMap::from([(
        ToleranceKey {
            tool: ToolId("bench".into()),
            measure: MeasureName("wall-ms".into()),
        },
        Percent(percent),
    )])
}

fn ran() -> Vec<ToolId> {
    vec![ToolId("bench".into())]
}

/// What an outcome says, as counts and the first median.
fn said(outcome: &PerfOutcome) -> String {
    format!(
        "regressed {} unseeded {} improved {} resolved {} held {}",
        outcome.regressed.len(),
        outcome.unseeded.len(),
        outcome.improved.len(),
        outcome.resolved.len(),
        outcome.held.0
    )
}

fn judged(value: f64, record: PerfRecord) -> String {
    said(&judge(&[measured(value)], &[record], &ran(), &none()))
}

fn written(records: &[PerfRecord]) -> String {
    records
        .iter()
        .map(|record| {
            format!(
                "{}/{} best {} {}",
                record.key.tool.0, record.key.measure.0, record.best.0, record.reasons[0].0
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_median_over_the_best_with_no_spread_is_regressed"),
        word: Word::Succeeds,
        input: || Question::Over,
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "regressed 1 unseeded 0 improved 0 resolved 0 held 0",
        )),
    },
    Case {
        name: CaseName("a_median_exactly_at_the_best_passes"),
        word: Word::Succeeds,
        input: || Question::AtBest,
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "regressed 0 unseeded 0 improved 0 resolved 0 held 1",
        )),
    },
    Case {
        name: CaseName("a_median_under_the_best_with_no_spread_is_an_improvement"),
        word: Word::Succeeds,
        input: || Question::Under,
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "regressed 0 unseeded 0 improved 1 resolved 0 held 0",
        )),
    },
    Case {
        name: CaseName("a_spread_with_its_low_end_over_the_best_is_regressed"),
        word: Word::Succeeds,
        input: || Question::SpreadAbove,
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "regressed 1 unseeded 0 improved 0 resolved 0 held 0",
        )),
    },
    Case {
        name: CaseName("a_spread_that_holds_the_best_passes"),
        word: Word::Succeeds,
        input: || Question::SpreadAround,
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "regressed 0 unseeded 0 improved 0 resolved 0 held 1",
        )),
    },
    Case {
        name: CaseName("a_spread_with_its_high_end_under_the_best_is_an_improvement"),
        word: Word::Succeeds,
        input: || Question::SpreadBelow,
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "regressed 0 unseeded 0 improved 1 resolved 0 held 0",
        )),
    },
    Case {
        name: CaseName("a_check_never_raises_a_best_whatever_its_spread_says"),
        word: Word::Succeeds,
        input: || Question::SpreadBelowValueAbove,
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "regressed 0 unseeded 0 improved 0 resolved 0 held 1",
        )),
    },
    Case {
        name: CaseName("a_median_over_the_best_by_less_than_the_tolerance_passes"),
        word: Word::Succeeds,
        input: || Question::WithinTolerance,
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "regressed 0 unseeded 0 improved 0 resolved 0 held 1",
        )),
    },
    Case {
        name: CaseName("a_median_over_the_best_by_exactly_the_tolerance_passes"),
        word: Word::Succeeds,
        input: || Question::AtTolerance,
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "regressed 0 unseeded 0 improved 0 resolved 0 held 1",
        )),
    },
    Case {
        name: CaseName("a_median_over_the_best_by_more_than_the_tolerance_is_regressed"),
        word: Word::Succeeds,
        input: || Question::BeyondTolerance,
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "regressed 1 unseeded 0 improved 0 resolved 0 held 0",
        )),
    },
    Case {
        name: CaseName("a_tolerance_does_not_stop_an_improvement"),
        word: Word::Succeeds,
        input: || Question::UnderWithTolerance,
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "regressed 0 unseeded 0 improved 1 resolved 0 held 0",
        )),
    },
    Case {
        name: CaseName("the_tolerance_of_another_measure_does_not_apply"),
        word: Word::Succeeds,
        input: || Question::ToleranceOfAnotherMeasure,
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "regressed 1 unseeded 0 improved 0 resolved 0 held 0",
        )),
    },
    Case {
        name: CaseName("a_median_with_no_record_is_unseeded"),
        word: Word::Succeeds,
        input: || Question::Unseeded,
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "regressed 0 unseeded 1 improved 0 resolved 0 held 0",
        )),
    },
    Case {
        name: CaseName("a_recorded_metric_the_tool_no_longer_reports_is_resolved"),
        word: Word::Succeeds,
        input: || Question::Dropped,
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "regressed 0 unseeded 0 improved 0 resolved 1 held 0",
        )),
    },
    Case {
        name: CaseName("a_record_of_a_tool_that_did_not_complete_is_not_resolved"),
        word: Word::Succeeds,
        input: || Question::ToolDown,
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "regressed 0 unseeded 0 improved 0 resolved 0 held 0",
        )),
    },
    Case {
        name: CaseName("an_improvement_rewrites_the_best_and_keeps_the_rest"),
        word: Word::Succeeds,
        input: || Question::Rewrite,
        expect: Expect::Value(std::borrow::Cow::Borrowed("bench/wall-ms best 70 seeded")),
    },
    Case {
        name: CaseName("a_regression_changes_a_passing_verdict"),
        word: Word::Succeeds,
        input: || Question::VerdictRegressed,
        expect: Expect::Value(std::borrow::Cow::Borrowed("Changed")),
    },
    Case {
        name: CaseName("an_improvement_alone_leaves_a_passing_verdict"),
        word: Word::Succeeds,
        input: || Question::VerdictImproved,
        expect: Expect::Value(std::borrow::Cow::Borrowed("Pass")),
    },
    Case {
        name: CaseName("a_regression_does_not_soften_an_undetermined_verdict"),
        word: Word::Succeeds,
        input: || Question::VerdictUndetermined,
        expect: Expect::Value(std::borrow::Cow::Borrowed("Undetermined")),
    },
    Case {
        name: CaseName("the_records_a_tool_no_longer_reports_are_named"),
        word: Word::Succeeds,
        input: || Question::Gone,
        expect: Expect::Value(std::borrow::Cow::Borrowed("1")),
    },
];

#[expect(
    clippy::unnecessary_wraps,
    reason = "every part's runner takes a subject that answers a Result, as `run` states it; these rows only succeed"
)]
fn subject(question: Input) -> Result<Value, Failure> {
    let outcome = |value: f64| judge(&[measured(value)], &[record()], &ran(), &none());
    let text = match question {
        Question::Over => judged(100.5, record()),
        Question::AtBest => judged(100.0, record()),
        Question::Under => judged(89.9, record()),
        Question::SpreadAbove => said(&judge(
            &[between(105.0, 115.0)],
            &[record()],
            &ran(),
            &none(),
        )),
        Question::SpreadAround => said(&judge(
            &[between(90.0, 110.0)],
            &[record()],
            &ran(),
            &none(),
        )),
        Question::SpreadBelow => said(&judge(&[between(80.0, 95.0)], &[record()], &ran(), &none())),
        Question::SpreadBelowValueAbove => said(&judge(
            &[spread(
                1000.0,
                Spread::Given(Number(1.0)),
                Spread::Given(Number(2.0)),
            )],
            &[record()],
            &ran(),
            &none(),
        )),
        Question::WithinTolerance => said(&judge(
            &[measured(104.0)],
            &[record()],
            &ran(),
            &allowing(5),
        )),
        Question::AtTolerance => said(&judge(
            &[measured(105.0)],
            &[record()],
            &ran(),
            &allowing(5),
        )),
        Question::BeyondTolerance => said(&judge(
            &[measured(105.5)],
            &[record()],
            &ran(),
            &allowing(5),
        )),
        Question::UnderWithTolerance => {
            said(&judge(&[measured(89.9)], &[record()], &ran(), &allowing(5)))
        }
        Question::ToleranceOfAnotherMeasure => said(&judge(
            &[measured(104.0)],
            &[record()],
            &ran(),
            &BTreeMap::from([(
                ToleranceKey {
                    tool: ToolId("bench".into()),
                    measure: MeasureName("cpu-ms".into()),
                },
                Percent(5),
            )]),
        )),
        Question::Unseeded => said(&judge(&[measured(100.0)], &[], &ran(), &none())),
        Question::Dropped => said(&judge(&[], &[record()], &ran(), &none())),
        Question::ToolDown => said(&judge(&[], &[record()], &[], &none())),
        Question::Rewrite => written(&rewritten(&outcome(70.0_f64))),
        Question::VerdictRegressed => {
            format!("{:?}", with_perf(Verdict::Pass, &outcome(500.0_f64)))
        }
        Question::VerdictImproved => format!("{:?}", with_perf(Verdict::Pass, &outcome(10.0_f64))),
        Question::VerdictUndetermined => {
            format!(
                "{:?}",
                with_perf(Verdict::Undetermined, &outcome(500.0_f64))
            )
        }
        Question::Gone => resolved(&[], &[record()], &ran()).len().to_string(),
    };
    Ok(std::borrow::Cow::Owned(text))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
