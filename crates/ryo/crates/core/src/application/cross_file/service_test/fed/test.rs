//! The rows each step of a rule was fed after the files, as the observer is told them: the vectors the interpreter reported for
//! the same scenarios (recorded on main at 262997fccb), a step that runs in each file fed nothing after the files.
use crate::application::cross_file::fixtures::evaluation::evaluate_fixture;
use crate::application::cross_file::fixtures::{counting_plan, frontier_rows, todo, world};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution::inbound::use_cases::execution::interface::ProgramRequestExecutionPort as _;
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::rule_requests::errors::RuleRequestsError;
use std::borrow::Cow;

/// The scenario a row runs.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// The nodes two files handed over, counted by one rule.
    TwoFilesCounted,
}

type Input = Given;
/// Every report the observer was told, in order, as text.
type Value = Cow<'static, str>;
type Failure = RuleRequestsError;

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("counts_are_the_interpreter_s"),
    word: Word::Succeeds,
    input: || Given::TwoFilesCounted,
    // main: `R4FED rule=0 fed=[RowsFed([0, 0, 0]), RowsFed([3, 0, 0])]` for cross_file::service_test::succeeds.
    expect: Expect::Value(Cow::Borrowed(
        "[(RuleIndex(0), [RowsFed([0, 0, 0]), RowsFed([3, 0, 0])])]",
    )),
}];

fn subject(given: Input) -> Result<Value, Failure> {
    match given {
        Given::TwoFilesCounted => {
            let rules = world(counting_plan(1, Vec::new()));
            let mut cross = Symbols::default();
            let phase = rules.request.project_phase().expect("one project phase");
            let _done = evaluate_fixture(
                &rules,
                &todo(0),
                &frontier_rows(),
                &rules.covered,
                &mut cross,
                &phase,
            )?;
            let reports = rules.observer.reports.lock().map_or_else(
                |_| "poisoned".to_owned(),
                |reports| format!("{:?}", *reports),
            );
            Ok(Cow::Owned(reports))
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
