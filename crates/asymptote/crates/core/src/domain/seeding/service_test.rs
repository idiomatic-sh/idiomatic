//! The perf records `up` writes: a median with no record seeds one, a run over its record's best raises the best and
//! appends the reason, and nothing else is written.

use super::service::seeding;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::operations::types::Reason;
use crate::domain::operations::types::{BenchmarkName, MeasureName, Number, PerfKey, PerfRecord};
use crate::domain::perf::types::{Measured, Spread};
use crate::domain::violations::types::ToolId;

/// Which question is asked.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// A median with no record.
    Seed,
    /// A median over its record's best, with no spread.
    Raise,
    /// A spread whose low end is over the record's best.
    RaiseBySpread,
    /// A spread whose low end is under the record's best.
    LeaveSpreadHolding,
    /// A median at its best.
    LeaveHeld,
    /// A median under its best.
    LeaveUnder,
    /// A tool that is not named.
    LeaveOtherTool,
    /// A tool that is named, among others.
    NamedTool,
}

type Input = Question;
type Value = std::borrow::Cow<'static, str>;
type Failure = String;

/// The record of `wall-ms` of tool `bench`: best 100.
fn record() -> PerfRecord {
    PerfRecord {
        key: PerfKey {
            tool: ToolId("bench".into()),
            benchmark: BenchmarkName("check".into()),
            measure: MeasureName("wall-ms".into()),
        },
        best: Number(100.0),
        reasons: vec![Reason("seeded".into())],
    }
}

fn measured(tool: &str, value: f64) -> Measured {
    spread(tool, value, Spread::Absent, Spread::Absent)
}

/// `value` of `wall-ms` of `tool`, reported with the bounds `lower` and `upper`.
fn spread(tool: &str, value: f64, lower: Spread, upper: Spread) -> Measured {
    Measured {
        key: PerfKey {
            tool: ToolId(tool.into()),
            benchmark: BenchmarkName("check".into()),
            measure: MeasureName("wall-ms".into()),
        },
        value: Number(value),
        lower,
        upper,
    }
}

/// What `up` writes, `tool/measure best reasons`, joined.
fn written(values: &[Measured], records: &[PerfRecord], tools: &[&str]) -> String {
    let tools: Vec<ToolId> = tools.iter().map(|tool| ToolId((*tool).into())).collect();
    seeding(values, records, &tools, &Reason("why".into()))
        .iter()
        .map(|record| {
            format!(
                "{}/{} best {} {}",
                record.key.tool.0,
                record.key.measure.0,
                record.best.0,
                record
                    .reasons
                    .iter()
                    .map(|reason| &*reason.0)
                    .collect::<Vec<_>>()
                    .join(",")
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_median_with_no_record_is_seeded_with_the_reason"),
        word: Word::Succeeds,
        input: || Question::Seed,
        expect: Expect::Value(std::borrow::Cow::Borrowed("bench/wall-ms best 120 why")),
    },
    Case {
        name: CaseName("a_median_over_the_best_raises_it_and_appends_the_reason"),
        word: Word::Succeeds,
        input: || Question::Raise,
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "bench/wall-ms best 120 seeded,why",
        )),
    },
    Case {
        name: CaseName("a_spread_with_its_low_end_over_the_best_raises_it_to_the_median"),
        word: Word::Succeeds,
        input: || Question::RaiseBySpread,
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "bench/wall-ms best 120 seeded,why",
        )),
    },
    Case {
        name: CaseName("a_spread_that_holds_the_best_writes_nothing"),
        word: Word::Succeeds,
        input: || Question::LeaveSpreadHolding,
        expect: Expect::Value(std::borrow::Cow::Borrowed("")),
    },
    Case {
        name: CaseName("a_median_at_its_best_writes_nothing"),
        word: Word::Succeeds,
        input: || Question::LeaveHeld,
        expect: Expect::Value(std::borrow::Cow::Borrowed("")),
    },
    Case {
        name: CaseName("a_median_under_its_best_writes_nothing"),
        word: Word::Succeeds,
        input: || Question::LeaveUnder,
        expect: Expect::Value(std::borrow::Cow::Borrowed("")),
    },
    Case {
        name: CaseName("the_medians_of_a_tool_not_named_are_left_alone"),
        word: Word::Succeeds,
        input: || Question::LeaveOtherTool,
        expect: Expect::Value(std::borrow::Cow::Borrowed("")),
    },
    Case {
        name: CaseName("the_medians_of_a_tool_named_are_written_and_the_others_are_not"),
        word: Word::Succeeds,
        input: || Question::NamedTool,
        expect: Expect::Value(std::borrow::Cow::Borrowed("bench/wall-ms best 120 why")),
    },
];

#[expect(
    clippy::unnecessary_wraps,
    reason = "every part's runner takes a subject that answers a Result, as `run` states it; these rows only succeed"
)]
fn subject(question: Input) -> Result<Value, Failure> {
    let text = match question {
        Question::Seed => written(&[measured("bench", 120.0)], &[], &[]),
        Question::Raise => written(&[measured("bench", 120.0)], &[record()], &["bench"]),
        Question::RaiseBySpread => written(
            &[spread(
                "bench",
                120.0,
                Spread::Given(Number(110.0)),
                Spread::Given(Number(130.0)),
            )],
            &[record()],
            &[],
        ),
        Question::LeaveSpreadHolding => written(
            &[spread(
                "bench",
                120.0,
                Spread::Given(Number(90.0)),
                Spread::Given(Number(150.0)),
            )],
            &[record()],
            &[],
        ),
        Question::LeaveHeld => written(&[measured("bench", 100.0)], &[record()], &[]),
        Question::LeaveUnder => written(&[measured("bench", 50.0)], &[record()], &[]),
        Question::LeaveOtherTool => written(&[measured("bench", 120.0)], &[], &["other"]),
        Question::NamedTool => written(
            &[measured("bench", 120.0), measured("other", 120.0)],
            &[],
            &["bench"],
        ),
    };
    Ok(std::borrow::Cow::Owned(text))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
